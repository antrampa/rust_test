use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------
// Config
// ---------------------------------------------------------------
#[derive(Debug)]
struct Config {
    host: String,
    port: u16,
    mount: String,
    password: String,
    name: String,
    bitrate: u64,
    folder_truck_e: String,
    folder_truck_1: String,
    folder_truck_3: String,
    folder_options: String,
}

fn load_config(path: &str) -> Config {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Cannot read config file: {}", path));

    let map: HashMap<String, String> = content
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut parts = l.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let val = parts.next()?.trim().to_string();
            Some((key, val))
        })
        .collect();

    let get = |key: &str| -> String {
        map.get(key)
            .unwrap_or_else(|| panic!("Missing config key: {}", key))
            .clone()
    };

    Config {
        host:           get("host"),
        port:           get("port").parse().expect("Invalid port"),
        mount:          get("mount"),
        password:       get("password"),
        name:           get("name"),
        bitrate:        get("bitrate").parse().expect("Invalid bitrate"),
        folder_truck_e: get("folders_truck_e"),
        folder_truck_1: get("folders_truck_1"),
        folder_truck_3: get("folders_truck_3"),
        folder_options: get("folders_options"),
    }
}

// ---------------------------------------------------------------
// Folder helpers
// ---------------------------------------------------------------
fn list_mp3s(folder: &str) -> Vec<PathBuf> {
    let path = Path::new(folder);
    if !path.exists() {
        eprintln!("Warning: folder does not exist: {}", folder);
        return vec![];
    }
    let mut files: Vec<PathBuf> = fs::read_dir(path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("mp3"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

fn shuffle<T>(v: &mut Vec<T>) {
    // Simple Fisher-Yates without external crates
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    let len = v.len();
    for i in (1..len).rev() {
        let j = (seed ^ (i * 2654435761)) % (i + 1);
        v.swap(i, j);
    }
}

fn pick_random(files: &[PathBuf]) -> Option<&PathBuf> {
    if files.is_empty() {
        return None;
    }
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    Some(&files[seed % files.len()])
}

fn pick_random_count(files: &[PathBuf], min: usize, max: usize) -> Vec<&PathBuf> {
    if files.is_empty() {
        return vec![];
    }
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    let count = min + (seed % (max - min + 1));
    let count = count.min(files.len());

    let mut indices: Vec<usize> = (0..files.len()).collect();
    shuffle(&mut indices);
    indices[..count].iter().map(|&i| &files[i]).collect()
}

// ---------------------------------------------------------------
// Base64
// ---------------------------------------------------------------
fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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

// ---------------------------------------------------------------
// Icecast connection
// ---------------------------------------------------------------
fn connect_and_authenticate(cfg: &Config) -> io::Result<TcpStream> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    println!("Connecting to Icecast at {}", addr);
    let mut stream = TcpStream::connect(&addr)?;

    let credentials = base64_encode(&format!("source:{}", cfg.password));
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
        cfg.mount, credentials, cfg.host, cfg.name, cfg.bitrate
    );

    stream.write_all(request.as_bytes())?;

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
            format!("Icecast rejected: {}", response.trim()),
        ));
    }

    println!("Authenticated! Starting stream...\n");
    Ok(stream)
}

// ---------------------------------------------------------------
// Update Icecast "Now Playing" metadata
// ---------------------------------------------------------------
fn update_metadata(cfg: &Config, title: &str) {
    // URL-encode the title (handles Greek and special chars)
    let encoded: String = title
        .bytes()
        .flat_map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b' ' {
                vec![b as char]
            } else {
                format!("%{:02X}", b).chars().collect::<Vec<char>>()
            }
        })
        .collect();

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let path = format!(
        "/admin/metadata?mount={}&mode=updinfo&song={}",
        cfg.mount, encoded
    );
    let credentials = base64_encode(&format!("source:{}", cfg.password));
    let request = format!(
        "GET {} HTTP/1.0\r\n\
         Authorization: Basic {}\r\n\
         Host: {}\r\n\
         User-Agent: RustStreamer/1.0\r\n\
         \r\n",
        path, credentials, cfg.host
    );

    // Fire and forget — don't crash the stream if metadata fails
    match TcpStream::connect(&addr) {
        Ok(mut s) => {
            let _ = s.write_all(request.as_bytes());
        }
        Err(e) => eprintln!("Metadata update failed: {}", e),
    }
}

// ---------------------------------------------------------------
// Stream a single MP3 file
// ---------------------------------------------------------------
fn stream_file(stream: &mut TcpStream, cfg: &Config, path: &PathBuf) -> io::Result<()> {
    // Extract filename without extension as the title
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    println!("▶ Now playing: {}", title);
    update_metadata(cfg, &format!("Currently playing: {}", title));

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buf = vec![0u8; 4096];

    let bytes_per_sec = (cfg.bitrate * 1000 / 8) as f64;
    let stream_start = Instant::now();
    let mut bytes_sent: u64 = 0;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buf[..n])?;
        bytes_sent += n as u64;

        // Absolute clock pacing — no drift accumulation
        let target_secs = bytes_sent as f64 / bytes_per_sec;
        let target_time = stream_start + Duration::from_secs_f64(target_secs);
        let now = Instant::now();
        if target_time > now {
            thread::sleep(target_time - now);
        }
    }

    println!("  Finished: {}\n", title);
    Ok(())
}

// ---------------------------------------------------------------
// Build the playlist sequence for one full loop cycle
// ---------------------------------------------------------------
fn build_cycle(cfg: &Config) -> Vec<PathBuf> {
    let mut cycle: Vec<PathBuf> = vec![];

    // Step 1: 3 tracks from truck-e (sequential, reshuffled each cycle)
    let mut truck_e = list_mp3s(&cfg.folder_truck_e);
    shuffle(&mut truck_e);
    for track in truck_e.iter().take(3) {
        cycle.push(track.clone());
    }

    // Step 2: 1 random track from truck-1
    let truck_1 = list_mp3s(&cfg.folder_truck_1);
    if let Some(t) = pick_random(&truck_1) {
        cycle.push(t.clone());
    }

    // Step 3: 2-5 random tracks from trucks-3
    let truck_3 = list_mp3s(&cfg.folder_truck_3);
    for t in pick_random_count(&truck_3, 2, 5) {
        cycle.push(t.clone());
    }

    // Step 4: 1 track from truck-options ONLY if folder has files
    let options = list_mp3s(&cfg.folder_options);
    if let Some(t) = pick_random(&options) {
        cycle.push(t.clone());
        println!("  [options track included this cycle]");
    } else {
        println!("  [no options tracks, skipping]");
    }

    cycle
}

// ---------------------------------------------------------------
// Main
// ---------------------------------------------------------------
fn main() {
    // Load config from same folder as the executable
    let config_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("config.txt")))
        .unwrap_or_else(|| PathBuf::from("config.txt"));

    let cfg = load_config(config_path.to_str().unwrap());
    println!("Loaded config from: {}", config_path.display());
    println!("Host: {}:{}{}", cfg.host, cfg.port, cfg.mount);

    loop {
        match connect_and_authenticate(&cfg) {
            Ok(mut stream) => {
                loop {
                    let cycle = build_cycle(&cfg);
                    println!("--- New cycle: {} tracks ---", cycle.len());

                    let mut cycle_ok = true;
                    for track in &cycle {
                        if let Err(e) = stream_file(&mut stream, &cfg, track) {
                            eprintln!("Stream error: {} — reconnecting...", e);
                            cycle_ok = false;
                            break;
                        }
                    }

                    if !cycle_ok {
                        break; // Break inner loop to reconnect
                    }
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {} — retrying in 5s...", e);
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}