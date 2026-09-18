//! Wire-level contract for `STORE` and `UID STORE`.
//!
//! Each test starts from a mailbox of three messages with known flags, issues a
//! STORE, then checks both the untagged FETCH the server answers with and - via
//! a follow-up FETCH - what actually got persisted.
//!
//! Flags are compared as sets, since the order of a flag list carries no
//! meaning.

use std::collections::BTreeSet;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};

mod test_util;
use test_util::{TestServer, read_available, start_server};

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

/// Log in, `APPEND` three messages into INBOX and `SELECT` it. Message 1 starts
/// with `\Seen`, message 2 with `\Seen \Flagged`, message 3 with no flags.
///
/// The starting flags go in through SQL rather than APPEND's flag list so these
/// tests only depend on STORE.
async fn login_with_messages(server: &TestServer) -> TcpStream {
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
    for (uid, flags) in [(1, vec!["\\Seen"]), (2, vec!["\\Seen", "\\Flagged"])] {
        let flags: Vec<String> = flags.into_iter().map(String::from).collect();
        sqlx::query!("UPDATE mail SET flags = $1 WHERE uid = $2", &flags, uid)
            .execute(&pool)
            .await
            .expect("failed to seed flags");
    }

    stream.write_all(b"a3 SELECT INBOX\r\n").unwrap();
    let resp = read_available(&mut stream);
    assert!(resp.contains("a3 OK"), "SELECT setup failed: {resp:?}");

    stream
}

/// Send `cmd` and return everything the server answers with.
fn send(stream: &mut TcpStream, cmd: &str) -> String {
    stream.write_all(cmd.as_bytes()).unwrap();
    read_available(stream)
}

/// The FLAGS list from the untagged `* {seqnum} FETCH` line in `resp`, or
/// `None` if there is no such line or it carries no FLAGS.
fn flags_for(resp: &str, seqnum: u32) -> Option<BTreeSet<String>> {
    let prefix = format!("* {seqnum} FETCH (");
    let line = resp.lines().find(|l| l.starts_with(&prefix))?;
    let start = line.find("FLAGS (")? + "FLAGS (".len();
    let end = start + line[start..].find(')')?;

    Some(
        line[start..end]
            .split_whitespace()
            .map(String::from)
            .collect(),
    )
}

fn set(flags: &[&str]) -> BTreeSet<String> {
    flags.iter().map(|f| f.to_string()).collect()
}

/// The flags message `seqnum` has persisted, read back through FETCH.
fn fetch_flags(stream: &mut TcpStream, seqnum: u32) -> BTreeSet<String> {
    let resp = send(stream, &format!("f1 FETCH {seqnum} FLAGS\r\n"));
    flags_for(&resp, seqnum).unwrap_or_else(|| panic!("no FLAGS for {seqnum}: {resp:?}"))
}

/// `FLAGS` throws away what was there and sets exactly the given list.
#[tokio::test(flavor = "multi_thread")]
async fn store_replace_sets_exactly_the_given_flags() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 2 FLAGS (\\Answered)\r\n");

    assert_eq!(
        flags_for(&resp, 2),
        Some(set(&["\\Answered"])),
        "expected untagged FETCH with the new flags: {resp:?}"
    );
    assert!(resp.contains("a4 OK"), "expected tagged OK: {resp:?}");
    assert!(
        !resp.contains("BAD"),
        "the whole command line should be consumed, leaving nothing to reparse: {resp:?}"
    );
    assert!(
        !resp.contains("UID"),
        "plain STORE addresses by seqnum, so no UID is owed: {resp:?}"
    );
    assert_eq!(fetch_flags(&mut stream, 2), set(&["\\Answered"]));
}

/// `FLAGS ()` clears every flag.
#[tokio::test(flavor = "multi_thread")]
async fn store_replace_with_empty_list_clears_flags() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 2 FLAGS ()\r\n");

    assert!(
        resp.contains("* 2 FETCH (FLAGS ())"),
        "expected `* 2 FETCH (FLAGS ())`: {resp:?}"
    );
    assert!(fetch_flags(&mut stream, 2).is_empty());
}

/// `+FLAGS` keeps the existing flags and adds the new ones.
#[tokio::test(flavor = "multi_thread")]
async fn store_add_keeps_existing_flags() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 1 +FLAGS (\\Deleted)\r\n");

    assert_eq!(
        flags_for(&resp, 1),
        Some(set(&["\\Seen", "\\Deleted"])),
        "expected untagged FETCH with the merged flags: {resp:?}"
    );
    assert_eq!(fetch_flags(&mut stream, 1), set(&["\\Seen", "\\Deleted"]));
}

/// Adding a flag the message already has doesn't duplicate it.
#[tokio::test(flavor = "multi_thread")]
async fn store_add_does_not_duplicate_flags() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 1 +FLAGS (\\Seen)\r\n");

    assert!(
        resp.contains("* 1 FETCH (FLAGS (\\Seen))"),
        "expected `\\Seen` exactly once: {resp:?}"
    );
}

/// Flags are case-insensitive: adding `\SEEN` to a `\Seen` message is a no-op,
/// and system flags come back in their canonical spelling.
#[tokio::test(flavor = "multi_thread")]
async fn store_add_matches_flags_case_insensitively() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 1 +FLAGS (\\SEEN \\flagged)\r\n");

    assert_eq!(
        flags_for(&resp, 1),
        Some(set(&["\\Seen", "\\Flagged"])),
        "expected canonical spellings, no duplicates: {resp:?}"
    );
}

/// Removing matches regardless of case, and non-system flags keep the
/// spelling the client first gave them.
#[tokio::test(flavor = "multi_thread")]
async fn store_remove_matches_flags_case_insensitively() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 3 +FLAGS (\\MyLabel)\r\n");
    assert_eq!(flags_for(&resp, 3), Some(set(&["\\MyLabel"])), "{resp:?}");

    send(&mut stream, "a5 STORE 2 +FLAGS (\\MyLabel)\r\n");
    let resp = send(&mut stream, "a6 STORE 2 -FLAGS (\\seen \\MYLABEL)\r\n");

    assert_eq!(
        flags_for(&resp, 2),
        Some(set(&["\\Flagged"])),
        "expected `\\Seen` and `\\MyLabel` gone: {resp:?}"
    );
}

/// `-FLAGS` removes only the named flags.
#[tokio::test(flavor = "multi_thread")]
async fn store_remove_only_removes_named_flags() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 2 -FLAGS (\\Seen)\r\n");

    assert_eq!(
        flags_for(&resp, 2),
        Some(set(&["\\Flagged"])),
        "expected only `\\Flagged` to remain: {resp:?}"
    );
    assert_eq!(fetch_flags(&mut stream, 2), set(&["\\Flagged"]));
}

/// Removing a flag the message doesn't have is not an error.
#[tokio::test(flavor = "multi_thread")]
async fn store_remove_absent_flag_is_a_no_op() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 1 -FLAGS (\\Draft)\r\n");

    assert_eq!(flags_for(&resp, 1), Some(set(&["\\Seen"])), "{resp:?}");
    assert!(resp.contains("a4 OK"), "expected tagged OK: {resp:?}");
}

/// Every message in the set is updated and gets its own untagged FETCH;
/// messages outside the set are untouched.
#[tokio::test(flavor = "multi_thread")]
async fn store_applies_to_every_message_in_the_set() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 1:2 +FLAGS (\\Draft)\r\n");

    assert_eq!(flags_for(&resp, 1), Some(set(&["\\Seen", "\\Draft"])));
    assert_eq!(
        flags_for(&resp, 2),
        Some(set(&["\\Seen", "\\Flagged", "\\Draft"]))
    );
    assert_eq!(flags_for(&resp, 3), None, "message 3 is not in 1:2: {resp:?}");
    assert!(fetch_flags(&mut stream, 3).is_empty());
}

/// `.SILENT` updates the flags but suppresses the untagged FETCH responses.
#[tokio::test(flavor = "multi_thread")]
async fn store_silent_sends_no_untagged_fetch() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    let resp = send(&mut stream, "a4 STORE 3 +FLAGS.SILENT (\\Seen)\r\n");

    assert!(
        !resp.contains("FETCH ("),
        "expected no untagged FETCH for .SILENT: {resp:?}"
    );
    assert!(resp.contains("a4 OK"), "expected tagged OK: {resp:?}");
    assert_eq!(fetch_flags(&mut stream, 3), set(&["\\Seen"]));
}

/// `UID STORE` addresses by UID. After dropping UID 1, UID 3 is sequence
/// number 2, and the response is keyed by that sequence number.
#[tokio::test(flavor = "multi_thread")]
async fn uid_store_addresses_by_uid() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    // EXPUNGE isn't implemented yet, so open the UID gap through SQL.
    let pool = server.db_pool().await;
    let deleted = sqlx::query!("DELETE FROM mail WHERE uid = 1")
        .execute(&pool)
        .await
        .expect("failed to drop the first message");
    assert_eq!(deleted.rows_affected(), 1, "expected to drop one message");

    let resp = send(&mut stream, "a4 UID STORE 3 FLAGS (\\Answered)\r\n");

    assert_eq!(
        flags_for(&resp, 2),
        Some(set(&["\\Answered"])),
        "expected UID 3 to be answered as seqnum 2: {resp:?}"
    );
    assert!(
        resp.lines()
            .any(|l| l.starts_with("* 2 FETCH (") && l.contains("UID 3")),
        "expected the UID in the untagged FETCH: {resp:?}"
    );
    assert!(resp.contains("a4 OK"), "expected tagged OK: {resp:?}");
    assert_eq!(fetch_flags(&mut stream, 2), set(&["\\Answered"]));
    assert_eq!(
        fetch_flags(&mut stream, 1),
        set(&["\\Seen", "\\Flagged"]),
        "UID 2 must be untouched"
    );
}

/// `STORE` must not be answerable before login.
#[tokio::test(flavor = "multi_thread")]
async fn store_before_login_is_rejected() {
    let addr: SocketAddr = *start_server().await;
    let mut stream = TcpStream::connect(addr).unwrap();
    let _ = read_available(&mut stream); // greeting

    let resp = send(&mut stream, "a1 STORE 1 FLAGS (\\Seen)\r\n");

    assert!(
        !resp.contains("FETCH ("),
        "expected no untagged FETCH before login: {resp:?}"
    );
    assert!(
        resp.contains("a1 BAD"),
        "expected a tagged BAD before login: {resp:?}"
    );
}

/// STATUS DELETED counts the messages a STORE marked `\Deleted`, and UNSEEN
/// the ones without `\Seen`.
#[tokio::test(flavor = "multi_thread")]
async fn status_counts_flags_set_by_store() {
    let server = start_server().await;
    let mut stream = login_with_messages(&server).await;

    send(&mut stream, "a4 STORE 3 +FLAGS.SILENT (\\Deleted)\r\n");
    let resp = send(&mut stream, "a5 STATUS INBOX (UNSEEN DELETED)\r\n");

    assert!(
        resp.contains("UNSEEN 1") && resp.contains("DELETED 1"),
        "expected one unseen and one deleted message: {resp:?}"
    );
}
