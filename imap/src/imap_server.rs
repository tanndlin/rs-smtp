use std::{
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread::{self, JoinHandle},
};

use sqlx::{Pool, Postgres};

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
    dbg!("Connection closed with peer: {addr}");
}
