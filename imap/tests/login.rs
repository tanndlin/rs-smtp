use std::io::Write;
use std::net::TcpStream;

mod test_util;
use test_util::{read_available, start_server};

#[tokio::test(flavor = "multi_thread")]
async fn responds_to_login() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // consume greeting

    stream.write_all(b"a1 LOGIN admin password\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("a1 OK LOGIN completed"),
        "failed to login: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "tagged completion line not CRLF-terminated: {resp:?}"
    );
}
