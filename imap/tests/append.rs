use std::io::Write;
use std::net::TcpStream;

mod test_util;
use test_util::{read_available, start_server};

/// Connect, consume the greeting, and authenticate. Returns the live stream.
fn connect_and_login(addr: std::net::SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // greeting

    stream
        .write_all(b"a1 LOGIN \"admin\" \"password\"\r\n")
        .unwrap();
    let resp = read_available(&mut stream);
    assert!(
        resp.contains("a1 OK LOGIN completed"),
        "login failed: {resp:?}"
    );
    stream
}

#[tokio::test(flavor = "multi_thread")]
async fn append_with_nonsync_literal_returns_ok() {
    let addr = start_server().await;
    let mut stream = connect_and_login(*addr);

    // LITERAL+ (RFC 7888): body follows immediately, no continuation needed.
    stream
        .write_all(b"a2 APPEND INBOX {13+}\r\nHello, World!\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("a2 OK") && resp.contains("APPEND completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn append_with_synchronizing_literal_gets_continuation() {
    let addr = start_server().await;
    let mut stream = connect_and_login(*addr);

    // No `+`: the server must reply with a `+` continuation before the client
    // sends the message octets.
    stream.write_all(b"a2 APPEND INBOX {13}\r\n").unwrap();
    let cont = read_available(&mut stream);
    assert!(
        cont.starts_with("+"),
        "expected `+` continuation request, got: {cont:?}"
    );

    stream.write_all(b"Hello, World!\r\n").unwrap();
    let resp = read_available(&mut stream);
    assert!(
        resp.contains("a2 OK") && resp.contains("APPEND completed"),
        "expected tagged OK after literal: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn append_accepts_flags_and_date_time() {
    let addr = start_server().await;
    let mut stream = connect_and_login(*addr);

    stream
        .write_all(
            b"a2 APPEND INBOX (\\Seen \\Draft) \"23-Oct-2024 19:00:00 +0000\" {13+}\r\nHello, World!\r\n",
        )
        .unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("a2 OK") && resp.contains("APPEND completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn append_before_login_is_rejected() {
    let addr = start_server().await;
    let mut stream = TcpStream::connect(*addr).unwrap();
    let _ = read_available(&mut stream); // greeting

    stream
        .write_all(b"a1 APPEND INBOX {13+}\r\nHello, World!\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("a1 NO") || resp.contains("a1 BAD"),
        "APPEND in unauthenticated state should be refused: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn append_to_unknown_mailbox_reports_trycreate() {
    let addr = start_server().await;
    let mut stream = connect_and_login(*addr);

    stream
        .write_all(b"a2 APPEND \"No Such Box\" {13+}\r\nHello, World!\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    // RFC 9051: appending to a non-existent mailbox SHOULD fail with a
    // `[TRYCREATE]` response code so the client can create it and retry.
    assert!(
        resp.contains("a2 NO") && resp.contains("TRYCREATE"),
        "expected `a2 NO [TRYCREATE]`: {resp:?}"
    );
}
