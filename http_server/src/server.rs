use std::{io::Read, net::TcpListener};

 pub struct Server {
    addr: String,
}

fn arr(a: &[u8]) {
    println!("array: {}", a[0]);
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }

    pub fn run(self) {
        println!("Listening on {}", self.addr);

        let listener = TcpListener::bind(&self.addr).unwrap();

        loop {

            match listener.accept() {
                Ok((mut stream, _)) => {
                   let a = [1,2,6,8,8,8,3,4,5];
                   arr(&a[0..3]);
                   //stream.read();
                },
                Err(e) => println!("Faild to establish a connection: {}", e),
            }
        }
    }
}