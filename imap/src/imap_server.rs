use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread::{self},
};

use sqlx::{Pool, Postgres};

use crate::imap_state::IMAPState;

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
    dbg!("Connection opened with peer: {addr}");

    // Send greeting
    stream.write_all(b"*OK IMAP4rev1 Server Ready\r\n").unwrap();

    let mut state = IMAPState::new();
    let mut buf = [0; 4096];
    while let Ok(bytes_read) = stream.read(&mut buf)
        && bytes_read > 0
    {
        println!("Read {bytes_read} bytes");
        let str = str::from_utf8(&buf).unwrap();
        println!("{str}")
    }

    #[cfg(debug_assertions)]
    dbg!("Connection closed with peer: {addr}");
}
