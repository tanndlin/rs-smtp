use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use imap::imap_server::IMAPServer;
use sqlx::postgres::PgPoolOptions;

// Lazy connect for fake postgres
fn start_server() -> SocketAddr {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused")
        .expect("failed to build lazy pool");

    let server = IMAPServer::new("127.0.0.1:0".parse().unwrap(), Arc::new(pool));
    let addr = server.local_addr().expect("no local addr");
    thread::spawn(move || server.start());
    addr
}

// Read whatever the server may have sent; do not block
fn read_available(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();

    let mut out = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(_) => break, // timeout: nothing more for now
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn sends_greeting_on_connect() {
    let addr = start_server();
    let mut stream = TcpStream::connect(addr).unwrap();

    let greeting = read_available(&mut stream);
    assert!(
        greeting.contains("OK") && greeting.ends_with("\r\n"),
        "unexpected greeting: {greeting:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn responds_to_capability() {
    let addr = start_server();
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // consume greeting

    stream.write_all(b"a1 CAPABILITY\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* CAPABILITY IMAP4rev2"),
        "missing capability listing: {resp:?}"
    );
    assert!(
        resp.contains("a1 OK CAPABILITY completed"),
        "missing tagged completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "tagged completion line not CRLF-terminated: {resp:?}"
    );
}
