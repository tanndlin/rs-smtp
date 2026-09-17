//! Wire-level contract for the `UID` commands.
//!
//! The point of `UID` is that its sequence sets address messages by UID, which
//! is stable, rather than by sequence number, which renumbers on every expunge.
//! These tests therefore work on a mailbox where the two have diverged, since a
//! mailbox that has never lost a message cannot tell the two readings apart.
//!
//! `UID STORE` and `UID COPY` parse but are still `todo!()` in the handler, so
//! only `UID FETCH` is covered here.

use std::io::Write;
use std::net::{SocketAddr, TcpStream};

mod test_util;
use test_util::{TestServer, read_available, start_server};

/// A complete RFC 5322 message. The subject is substituted per copy so each
/// appended message is distinguishable in a FETCH response.
fn message(subject: &str) -> Vec<u8> {
    format!(
        "Date: Wed, 23 Oct 2024 19:00:00 +0000\r\n\
         From: alice@example.com\r\n\
         To: bob@example.com\r\n\
         Subject: {subject}\r\n\
         Message-ID: <{subject}@example.com>\r\n\
         \r\n\
         Hello, Bob.\r\n"
    )
    .into_bytes()
}

/// Log in, `APPEND` three messages into INBOX, drop the first one, then
/// `SELECT`. The mailbox is left holding two messages whose sequence numbers
/// (1, 2) and UIDs (2, 3) disagree.
///
/// The drop goes through SQL because EXPUNGE isn't implemented yet; what matters
/// to these tests is the resulting UID gap, not how it got there.
async fn login_with_uid_gap(server: &TestServer) -> TcpStream {
    let mut stream = TcpStream::connect(server.addr).unwrap();
    let _ = read_available(&mut stream); // greeting

    stream
        .write_all(b"a1 LOGIN \"admin\" \"password\"\r\n")
        .unwrap();
    let resp = read_available(&mut stream);
    assert!(
        resp.contains("a1 OK LOGIN completed"),
        "login failed: {resp:?}"
    );

    for subject in ["first", "second", "third"] {
        let raw = message(subject);
        let mut append = format!("a2 APPEND INBOX {{{}+}}\r\n", raw.len()).into_bytes();
        append.extend_from_slice(&raw);
        append.extend_from_slice(b"\r\n");
        stream.write_all(&append).unwrap();
        let resp = read_available(&mut stream);
        assert!(resp.contains("a2 OK"), "APPEND setup failed: {resp:?}");
    }

    let pool = server.db_pool().await;
    let deleted = sqlx::query!("DELETE FROM mail WHERE uid = 1")
        .execute(&pool)
        .await
        .expect("failed to drop the first message");
    assert_eq!(deleted.rows_affected(), 1, "expected to drop one message");

    stream.write_all(b"a3 SELECT INBOX\r\n").unwrap();
    let resp = read_available(&mut stream);
    assert!(resp.contains("a3 OK"), "SELECT setup failed: {resp:?}");

    stream
}

/// A UID above the message count still addresses its message. Reading the set
/// as sequence numbers would clamp 3 away on a two-message mailbox and answer
/// with no data at all.
#[tokio::test(flavor = "multi_thread")]
async fn uid_fetch_addresses_by_uid_not_sequence_number() {
    let server = start_server().await;
    let mut stream = login_with_uid_gap(&server).await;

    stream.write_all(b"a4 UID FETCH 3 RFC822.SIZE\r\n").unwrap();
    let resp = read_available(&mut stream);

    let size = message("third").len();
    assert!(
        resp.contains("* 2 FETCH (") && resp.contains("UID 3"),
        "expected untagged FETCH for seqnum 2 / UID 3: {resp:?}"
    );
    assert!(
        resp.contains(&format!("RFC822.SIZE {size}")),
        "expected the third message's size {size}: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// `1:*` is the resync every client issues. `*` has to mean the largest UID, so
/// the whole mailbox comes back even though both UIDs exceed the message count.
#[tokio::test(flavor = "multi_thread")]
async fn uid_fetch_wild_range_covers_the_whole_mailbox() {
    let server = start_server().await;
    let mut stream = login_with_uid_gap(&server).await;

    stream.write_all(b"a4 UID FETCH 1:* UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (UID 2)"),
        "expected untagged `* 1 FETCH (UID 2)`: {resp:?}"
    );
    assert!(
        resp.contains("* 2 FETCH (UID 3)"),
        "expected untagged `* 2 FETCH (UID 3)`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// The UID data item comes back whether or not the client asked for it, since
/// the client has no sequence number to key the response on otherwise.
#[tokio::test(flavor = "multi_thread")]
async fn uid_fetch_includes_uid_unrequested() {
    let server = start_server().await;
    let mut stream = login_with_uid_gap(&server).await;

    stream.write_all(b"a4 UID FETCH 2 FLAGS\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("UID 2"),
        "expected UID 2 in the response despite only FLAGS being asked for: {resp:?}"
    );
    assert!(
        resp.contains("FLAGS ()"),
        "expected the requested FLAGS item: {resp:?}"
    );
}

/// A UID matching no message is skipped silently - no untagged data, still a
/// tagged OK. This is what lets a client fetch a UID another session expunged.
#[tokio::test(flavor = "multi_thread")]
async fn uid_fetch_of_missing_uid_is_not_an_error() {
    let server = start_server().await;
    let mut stream = login_with_uid_gap(&server).await;

    stream.write_all(b"a4 UID FETCH 1 UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        !resp.contains("FETCH ("),
        "expected no untagged FETCH for the expunged UID 1: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// The plain FETCH counterpart: sequence number 1 is the mailbox's first
/// message, which now carries UID 2.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_addresses_by_sequence_number_after_a_gap() {
    let server = start_server().await;
    let mut stream = login_with_uid_gap(&server).await;

    stream.write_all(b"a4 FETCH 1 UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (UID 2)"),
        "expected untagged `* 1 FETCH (UID 2)`: {resp:?}"
    );
}

/// `UID FETCH` must not be answerable before login.
#[tokio::test(flavor = "multi_thread")]
async fn uid_fetch_before_login_is_rejected() {
    let addr: SocketAddr = *start_server().await;
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // greeting

    stream.write_all(b"a1 UID FETCH 1 UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        !resp.contains("FETCH ("),
        "expected no untagged FETCH before login: {resp:?}"
    );
    assert!(
        resp.contains("a1 BAD"),
        "expected a tagged BAD before login: {resp:?}"
    );
}
