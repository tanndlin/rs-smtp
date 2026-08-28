use std::io::Write;
use std::net::TcpStream;

mod test_util;
use test_util::{read_available, start_server};

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
