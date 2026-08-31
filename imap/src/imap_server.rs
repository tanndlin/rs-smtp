use std::{net::SocketAddr, sync::Arc};

use sqlx::{Pool, Postgres};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{command::ClientCommand, imap_session::IMAPSession};
use util::EncodeTo;

pub struct IMAPServer {
    db_pool: Arc<Pool<Postgres>>,
    listener: TcpListener,
}

impl IMAPServer {
    pub async fn new(ip: SocketAddr, db_pool: Arc<Pool<Postgres>>) -> Self {
        let listener = TcpListener::bind(ip)
            .await
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

    pub async fn start(&self) {
        loop {
            let (stream, addr) = self.listener.accept().await.unwrap();
            let db_pool = self.db_pool.clone();
            tokio::spawn(handle_request(stream, addr, db_pool));
        }
    }
}

async fn handle_request(mut stream: TcpStream, addr: SocketAddr, db_pool: Arc<Pool<Postgres>>) {
    #[cfg(debug_assertions)]
    println!("Connection opened with peer: {addr}");

    // Send greeting
    let mut state = IMAPSession::new(db_pool);
    let res = state.send_greeting();
    stream.write_all(&res.to_bytes()).await.unwrap();

    let mut bytes = vec![];
    let mut buf = [0; 4096];
    while let Ok(bytes_read) = stream.read(&mut buf).await
        && bytes_read > 0
    {
        println!("Read {bytes_read} bytes");
        bytes.extend_from_slice(&buf[..bytes_read]);

        // TODO: This will probably break for multiline commands
        while bytes.windows(2).any(|window| window == b"\r\n") {
            match ClientCommand::parse_bytes(&bytes) {
                Ok((command, read)) => {
                    bytes.drain(0..read);
                    dbg!(&command);
                    let res = state.handle_command(command).await;
                    dbg!(&res);
                    stream.write_all(&res.to_bytes()).await.unwrap();

                    if state.is_logged_out() {
                        let _ = stream.shutdown().await;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to parse command from {addr}:\n{}",
                        e.render(&bytes)
                    );
                    let _ = stream
                        .write_all(b"* BAD could not parse command\r\n")
                        .await;
                    // Drop what we have so we don't spin on the same bytes.
                    bytes.clear();
                    break;
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    dbg!("Connection closed with peer: {addr}");
}
