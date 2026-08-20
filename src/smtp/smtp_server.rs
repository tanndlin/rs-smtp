use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

use crate::{
    smtp::message::{Message, Ready},
    util::encode_to::EncodeTo,
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

fn handle_request(mut stream: TcpStream, addr: SocketAddr) {
    println!("Connected to client: {addr}");

    let ready = Message::Ready(Ready::new());
    ready.write_to(&mut stream).unwrap();

    let mut buf = [0u8; 1024];
    let bytes_read = stream.read(&mut buf).unwrap();
    let string = str::from_utf8(&buf[..bytes_read]).unwrap();
    println!("Read {bytes_read} bytes: {string}");
}
