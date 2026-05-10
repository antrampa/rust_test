use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    println!("Rust TCP Port Tester");
    let addr = "127.0.0.1:8080".to_string();
    println!("Connecting to the address {}", addr);

    if let Ok(stream) = TcpStream::connect(&addr) {
        println!("Connected to the server!");

        // Clone the stream for the reader thread
        let stream = Arc::new(Mutex::new(stream));
        let stream_reader = Arc::clone(&stream);

        // Spawn a thread to read incoming data from the server
        let reader_thread = thread::spawn(move || {
            let mut buf = vec![0u8; 1024];
            loop {
                let mut s = stream_reader.lock().unwrap();
                s.set_nonblocking(true).expect("set_nonblocking failed");
                match s.read(&mut buf) {
                    Ok(0) => {
                        println!("\n[Server closed the connection]");
                        break;
                    }
                    Ok(n) => {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        println!("\n[Server]: {}", msg);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // No data yet, just continue
                        drop(s);
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("Read error: {e}");
                        break;
                    }
                }
            }
        });

        // Main thread: read user input and send to server
        println!("Type a message and press Enter to send. Ctrl+C to quit.");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(input) => {
                    let message = format!("{}\n", input);
                    let mut s = stream.lock().unwrap();
                    if let Err(e) = s.write_all(message.as_bytes()) {
                        eprintln!("Write error: {e}");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Input error: {e}");
                    break;
                }
            }
        }

        reader_thread.join().unwrap();
    } else {
        println!("Couldn't connect to server...");
    }
}