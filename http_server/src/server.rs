 use std::net::TcpListener;

 pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }

    pub fn run(self) -> (i32, &str, std::net::TcpListener) {
        println!("Listening on {}", self.addr);

        let listener = TcpListener::bind(&self.addr).unwrap();

        loop {
            listener.accept();
        }

        let tup = (5, "a", listener);

    }
}