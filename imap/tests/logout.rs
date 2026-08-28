use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

mod test_util;
use test_util::{read_available, start_server};

#[tokio::test(flavor = "multi_thread")]
async fn logout_closes_connection() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // consume greeting

    stream.write_all(b"a1 LOGOUT\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(resp.contains("* BYE"), "missing BYE: {resp:?}");
    assert!(
        resp.contains("a1 OK LOGOUT completed"),
        "missing tagged OK: {resp:?}"
    );

    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut buf = [0u8; 16];
    let n = stream.read(&mut buf).unwrap();
    assert_eq!(n, 0, "expected EOF after LOGOUT, server kept connection open");
}
