use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

const ICECAST_HOST: &str = "127.0.0.1";
const ICECAST_PORT: u16 = 8000;
const ICECAST_PORT: u16 = 8080;
const ICECAST_MOUNT: &str = "/stream";
const ICECAST_PASSWORD: &str = "hackme";
const ICECAST_NAME: &str = "My Rust Stream";
const BITRATE: u64 = 128; // kbps, must match your MP3 files
const CHUNK_SIZE: usize = 4096;

fn connect_and_authenticate() -> io::Result<TcpStream> {
    let addr = format!("{}:{}", ICECAST_HOST, ICECAST_PORT);
    println!("Connecting to Icecast at {}", addr);
    let mut stream = TcpStream::connect(&addr)?;

    // Icecast uses HTTP PUT to start a source connection
    let credentials = base64_encode(&format!("source:{}", ICECAST_PASSWORD));
    let request = format!(
        "PUT {} HTTP/1.0\r\n\
         Authorization: Basic {}\r\n\
         Host: {}\r\n\
         User-Agent: RustStreamer/1.0\r\n\
         Accept: */*\r\n\
         Transfer-Encoding: chunked\r\n\
         Content-Type: audio/mpeg\r\n\
         Ice-Name: {}\r\n\
         Ice-BitRate: {}\r\n\
         Ice-Public: 0\r\n\
         \r\n",
        ICECAST_MOUNT, credentials, ICECAST_HOST, ICECAST_NAME, BITRATE
    );

    stream.write_all(request.as_bytes())?;

    // Read the HTTP response from Icecast
    let mut response = String::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        response.push(byte[0] as char);
        if response.ends_with("\r\n\r\n") {
            break;
        }
    }

    println!("Icecast response:\n{}", response.trim());

    if !response.contains("200 OK") {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Icecast rejected connection: {}", response.trim()),
        ));
    }

    println!("Authenticated! Starting stream...");
    Ok(stream)
}

/// Minimal base64 encoder (avoids needing a dependency)
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;

    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };

        result.push(CHARS[((b0 >> 2) & 0x3F) as usize] as char);
        result.push(CHARS[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize] as char);
        result.push(if i + 1 < bytes.len() { CHARS[(((b1 << 2) | (b2 >> 6)) & 0x3F) as usize] as char } else { '=' });
        result.push(if i + 2 < bytes.len() { CHARS[(b2 & 0x3F) as usize] as char } else { '=' });

        i += 3;
    }
    result
}

fn stream_file_v1(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    println!("Streaming: {}", path);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = vec![0u8; CHUNK_SIZE];

    // How long each chunk should take to send, based on bitrate
    // bitrate (kbps) -> bytes per second = bitrate * 1000 / 8
    let bytes_per_sec = (BITRATE * 1000 / 8) as f64;
    let secs_per_chunk = CHUNK_SIZE as f64 / bytes_per_sec;
    let delay_per_chunk = Duration::from_secs_f64(secs_per_chunk);

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break; // EOF
        }

        let start = Instant::now();
        stream.write_all(&buf[..n])?;

        // Pace the stream to match the bitrate so Icecast doesn't buffer-overflow
        let elapsed = start.elapsed();
        if delay_per_chunk > elapsed {
            thread::sleep(delay_per_chunk - elapsed);
        }
    }

    println!("Finished: {}", path);
    Ok(())
}

fn stream_file(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    println!("Streaming: {}", path);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = vec![0u8; CHUNK_SIZE];

    let bytes_per_sec = (BITRATE * 1000 / 8) as f64;
    let secs_per_chunk = CHUNK_SIZE as f64 / bytes_per_sec;

    let stream_start = Instant::now();
    let mut bytes_sent: u64 = 0;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }

        stream.write_all(&buf[..n])?;
        bytes_sent += n as u64;

        // Calculate exactly when this byte position should be reached
        let target_secs = bytes_sent as f64 / bytes_per_sec;
        let target_time = stream_start + Duration::from_secs_f64(target_secs);
        let now = Instant::now();

        if target_time > now {
            thread::sleep(target_time - now);
        }
    }

    println!("Finished: {}", path);
    Ok(())
}

fn main() {
    // List of MP3 files to stream in order (loops forever)
    let playlist = vec![
        "track3.mp3",
        "track2.mp3",
        "track1.mp3",
    ];

    loop {
        match connect_and_authenticate() {
            Ok(mut stream) => {
                for &file in &playlist {
                    match stream_file(&mut stream, file) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error streaming {}: {}", file, e);
                            break; // Reconnect on error
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {}. Retrying in 5s...", e);
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}