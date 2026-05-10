use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {
    // let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    // stream.write(&[1])?;
    // stream.read(&mut [0; 128])?;
    // Ok(())


    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:8080") {
        println!("Connected to the server!");
        stream.write(&[1])?;
        stream.write_all("message".as_bytes());
    } else {
        println!("Couldn't connect to server...");
    }

    return Ok(());
} // the stream is closed here