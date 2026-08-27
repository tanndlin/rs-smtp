use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread::{self},
};

use sqlx::{Pool, Postgres};

use crate::{command::ClientCommand, imap_state::IMAPState};
use util::EncodeTo;

pub struct IMAPServer {
    db_pool: Arc<Pool<Postgres>>,
    listener: TcpListener,
}

impl IMAPServer {
    pub fn new(ip: SocketAddr, db_pool: Arc<Pool<Postgres>>) -> Self {
        let listener = TcpListener::bind(ip)
            .map_err(|e| format!("Error creating tcp listener {e}"))
            .unwrap();

        println!("Listening on {ip}");

        Self { db_pool, listener }
    }

    /// The address the listener is actually bound to. Useful when constructed
    /// with port 0 (e.g. in tests) so the OS-assigned port can be discovered.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn start(&self) {
        loop {
            let (stream, addr) = self.listener.accept().unwrap();
            let db_pool = self.db_pool.clone();
            thread::spawn(move || handle_request(stream, addr, db_pool));
        }
    }
}

fn handle_request(mut stream: TcpStream, addr: SocketAddr, db_pool: Arc<Pool<Postgres>>) {
    #[cfg(debug_assertions)]
    println!("Connection opened with peer: {addr}");

    // Send greeting
    let mut state = IMAPState::new();
    let res = state.send_greeting();
    stream.write_all(&res.to_bytes()).unwrap();

    let mut bytes = vec![];
    let mut buf = [0; 4096];
    while let Ok(bytes_read) = stream.read(&mut buf)
        && bytes_read > 0
    {
        println!("Read {bytes_read} bytes");
        bytes.extend_from_slice(&buf[..bytes_read]);
        match ClientCommand::parse_bytes(&bytes) {
            Ok((command, read)) => {
                bytes.drain(0..read);
                dbg!(&command);
                let res = state.handle_command(command);
                stream.write_all(&res.to_bytes()).unwrap();
            }
            Err(e) => todo!("{:?}", e),
        }
    }

    #[cfg(debug_assertions)]
    dbg!("Connection closed with peer: {addr}");
}
