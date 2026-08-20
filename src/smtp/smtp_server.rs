use std::{
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    thread::{self, JoinHandle, Thread},
};

pub struct SMTPServer {
    listen_thread: JoinHandle<()>,
}

impl SMTPServer {
    pub fn new(addr: SocketAddr) -> Result<Self, String> {
        let listener =
            TcpListener::bind(addr).map_err(|e| format!("Error creating tcp listener {e}"))?;

        println!("Listening on {addr}");
        let listen_thread = thread::spawn(|| listen(listener));

        Ok(Self { listen_thread })
    }

    pub fn join(self) {
        self.listen_thread.join().unwrap();
    }
}

fn listen(listener: TcpListener) {
    loop {
        let (stream, addr) = listener.accept().unwrap();
        thread::spawn(move || handle_request(stream, addr));
    }
}

fn handle_request(stream: TcpStream, addr: SocketAddr) {
    println!("Connected to client: {addr}")
}
