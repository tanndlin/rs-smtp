//! Wire-level contract for `FETCH` response handling.
//!
//! The base handler is in place: `UID`, `RFC822.SIZE`, `ENVELOPE`, `FLAGS`,
//! `BODY[]` and `BODY[HEADER]` round-trip. Tests suffixed `_red` (and the
//! block of them below `fetch_body_peek_header_returns_headers`) pin
//! behaviour that is still `todo!()` in `get_fetchable` / `handle_body`
//! (`BODY[TEXT]`, header-field subsets, numbered parts, partials,
//! `BODYSTRUCTURE`, `INTERNALDATE`, the `ALL` macro) and are expected to be
//! RED until each arm lands. They pin the RFC 9051 §7.5.2 response shape a
//! client should see.

use std::io::Write;
use std::net::{SocketAddr, TcpStream};

mod test_util;
use test_util::{read_available, start_server};

/// A complete RFC 5322 message: header block, blank-line separator, one line of
/// body. Appended verbatim and then fetched back.
const MESSAGE: &[u8] = b"Date: Wed, 23 Oct 2024 19:00:00 +0000\r\n\
From: alice@example.com\r\n\
To: bob@example.com\r\n\
Subject: greetings\r\n\
Message-ID: <abc123@example.com>\r\n\
\r\n\
Hello, Bob.\r\n";

/// A second message that populates every `ENVELOPE` address field with a
/// distinct address, so `Sender`/`Reply-To`/`Cc`/`Bcc`/`In-Reply-To` can be
/// checked against real header values instead of their From-derived/NIL
/// defaults.
const FULL_MESSAGE: &[u8] = b"Date: Thu, 24 Oct 2024 08:15:00 +0000\r\n\
From: alice@example.com\r\n\
Sender: carol@example.com\r\n\
Reply-To: dave@example.com\r\n\
To: bob@example.com\r\n\
Cc: erin@example.com\r\n\
Bcc: frank@example.com\r\n\
In-Reply-To: <root123@example.com>\r\n\
Subject: re: greetings\r\n\
Message-ID: <def456@example.com>\r\n\
\r\n\
Hi Alice.\r\n";

/// Connect, consume the greeting, and authenticate. Returns the live stream.
fn connect_and_login(addr: SocketAddr) -> TcpStream {
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

/// Log in, `APPEND` `raw` into INBOX (via a `LITERAL+` non-sync literal), and
/// `SELECT` INBOX. The appended message is then sequence number 1 / UID 1 in a
/// freshly migrated database.
fn login_with_message(addr: SocketAddr, raw: &[u8]) -> TcpStream {
    let mut stream = connect_and_login(addr);

    let mut append = format!("a2 APPEND INBOX {{{}+}}\r\n", raw.len()).into_bytes();
    append.extend_from_slice(raw);
    append.extend_from_slice(b"\r\n");
    stream.write_all(&append).unwrap();
    let resp = read_available(&mut stream);
    assert!(resp.contains("a2 OK"), "APPEND setup failed: {resp:?}");

    stream.write_all(b"a3 SELECT INBOX\r\n").unwrap();
    let resp = read_available(&mut stream);
    assert!(resp.contains("a3 OK"), "SELECT setup failed: {resp:?}");

    stream
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_uid_returns_uid() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (UID 1)"),
        "expected untagged `* 1 FETCH (UID 1)`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_last_returns_uid() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH * UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (UID 1)"),
        "expected untagged `* 1 FETCH (UID 1)`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_rfc822_size_returns_size() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 RFC822.SIZE\r\n").unwrap();
    let resp = read_available(&mut stream);

    let size = MESSAGE.len();
    assert!(
        resp.contains(&format!("* 1 FETCH (RFC822.SIZE {size})")),
        "expected untagged `* 1 FETCH (RFC822.SIZE {size})`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_envelope() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 ENVELOPE\r\n").unwrap();
    let resp = read_available(&mut stream);

    // RFC 9051 §7.5.2: env-structure is (date subject from sender reply-to
    // to cc bcc in-reply-to message-id). MESSAGE has no Sender or Reply-To
    // header, so both default to the From address; it has no Cc, Bcc, or
    // In-Reply-To header, so those are NIL.
    let expected = "* 1 FETCH (ENVELOPE (\"Wed, 23 Oct 2024 19:00:00 +0000\" \"greetings\" \
        ((NIL NIL \"alice\" \"example.com\")) \
        ((NIL NIL \"alice\" \"example.com\")) \
        ((NIL NIL \"alice\" \"example.com\")) \
        ((NIL NIL \"bob\" \"example.com\")) \
        NIL NIL NIL \"<abc123@example.com>\"))";
    assert!(
        resp.contains(expected),
        "expected untagged `{expected}`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_envelope_with_all_fields() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, FULL_MESSAGE);

    stream.write_all(b"a4 FETCH 1 ENVELOPE\r\n").unwrap();
    let resp = read_available(&mut stream);

    // Every address field comes from its own header this time, and
    // In-Reply-To is present, so nothing falls back to a default or NIL.
    let expected = "* 1 FETCH (ENVELOPE (\"Thu, 24 Oct 2024 08:15:00 +0000\" \"re: greetings\" \
        ((NIL NIL \"alice\" \"example.com\")) \
        ((NIL NIL \"carol\" \"example.com\")) \
        ((NIL NIL \"dave\" \"example.com\")) \
        ((NIL NIL \"bob\" \"example.com\")) \
        ((NIL NIL \"erin\" \"example.com\")) \
        ((NIL NIL \"frank\" \"example.com\")) \
        \"<root123@example.com>\" \"<def456@example.com>\"))";
    assert!(
        resp.contains(expected),
        "expected untagged `{expected}`: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
    assert!(
        resp.ends_with("\r\n"),
        "completion line not CRLF-terminated: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_flags_empty_by_default() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    // The message was appended with no flag list, so it has no flags.
    stream.write_all(b"a4 FETCH 1 FLAGS\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (FLAGS ())"),
        "expected an empty FLAGS list: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_full_body_returns_raw_eml() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODY[]\r\n").unwrap();
    let resp = read_available(&mut stream);

    // RFC 9051: `BODY[]` is returned as a literal — `BODY[] {<octets>}\r\n<data>`.
    assert!(
        resp.contains("BODY[] {"),
        "expected a `BODY[] {{n}}` literal: {resp:?}"
    );
    // The literal payload is the message exactly as appended.
    assert!(
        resp.contains("Subject: greetings") && resp.contains("Hello, Bob."),
        "literal did not carry the whole raw message: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// `FETCH 1 (UID RFC822.SIZE)` - a parenthesised att list returns both items
/// on a single untagged line. Ordering inside the parens is not asserted.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_uid_and_size_together() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream
        .write_all(b"a4 FETCH 1 (UID RFC822.SIZE)\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    let size = MESSAGE.len();
    assert_eq!(
        resp.matches("* 1 FETCH (").count(),
        1,
        "expected exactly one untagged FETCH line: {resp:?}"
    );
    assert!(resp.contains("UID 1"), "missing UID item: {resp:?}");
    assert!(
        resp.contains(&format!("RFC822.SIZE {size}")),
        "missing RFC822.SIZE item: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `BODY[TEXT]` should return the body only, as a `BODY[TEXT]` literal.
/// `handle_body` still has `todo!()` for `SectionText::Text`.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_body_text_returns_body_only() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODY[TEXT]\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("BODY[TEXT] {"),
        "expected a `BODY[TEXT] {{n}}` literal: {resp:?}"
    );
    assert!(
        resp.contains("Hello, Bob."),
        "literal did not carry the body text: {resp:?}"
    );
    assert!(
        !resp.contains("Subject: greetings"),
        "`BODY[TEXT]` must not include headers: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `BODY[HEADER.FIELDS (SUBJECT)]` should return just the named header.
/// `handle_body` still has `todo!()` for `SectionText::HeaderFields`.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_body_header_fields_returns_subset() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream
        .write_all(b"a4 FETCH 1 BODY.PEEK[HEADER.FIELDS (SUBJECT)]\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("BODY[HEADER.FIELDS (SUBJECT)] {"),
        "expected a `BODY[HEADER.FIELDS (SUBJECT)] {{n}}` literal: {resp:?}"
    );
    assert!(
        resp.contains("Subject: greetings"),
        "subset missing the requested header: {resp:?}"
    );
    assert!(
        !resp.contains("From: alice@example.com"),
        "subset must not include unrequested headers: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `BODY[1]` on a non-multipart message is the message body.
/// `handle_body` still has `todo!()` for `Section::Part`.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_numbered_body_part_returns_part() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODY[1]\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("BODY[1] {"),
        "expected a `BODY[1] {{n}}` literal: {resp:?}"
    );
    assert!(
        resp.contains("Hello, Bob."),
        "part literal did not carry the body: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: a partial `BODY[]<0.10>` returns only the first ten octets, and the
/// response item names the origin octet: `BODY[]<0> {10}`. `handle_body`
/// ignores the partial range today and returns the whole message.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_body_partial_returns_truncated_octets() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODY[]<0.10>\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("BODY[]<0> {10}"),
        "expected a `BODY[]<0> {{10}}` partial literal: {resp:?}"
    );
    assert!(
        !resp.contains("Hello, Bob."),
        "partial must not carry the whole message body: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: bare `FETCH 1 BODY` returns the non-extensible BODYSTRUCTURE.
/// `handle_body` still has `todo!()` for `BodyFetchable::Full`.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_bare_body_returns_body_structure() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODY\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (BODY ("),
        "expected a `BODY (...)` structure: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `FETCH 1 BODYSTRUCTURE` is unimplemented (`todo!()` in `get_fetchable`).
#[tokio::test(flavor = "multi_thread")]
async fn fetch_bodystructure_returns_structure() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 BODYSTRUCTURE\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (BODYSTRUCTURE ("),
        "expected a `BODYSTRUCTURE (...)` structure: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `FETCH 1 INTERNALDATE` is unimplemented (`todo!()` in `get_fetchable`).
#[tokio::test(flavor = "multi_thread")]
async fn fetch_internaldate_returns_quoted_date() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 INTERNALDATE\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        resp.contains("* 1 FETCH (INTERNALDATE \""),
        "expected a quoted `INTERNALDATE` value: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: the `FETCH 1 ALL` macro expands to
/// `(FLAGS INTERNALDATE RFC822.SIZE ENVELOPE)`. `get_fetchable` has `todo!()`
/// for `Fetchable::All`.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_all_macro_expands_to_four_items() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream.write_all(b"a4 FETCH 1 ALL\r\n").unwrap();
    let resp = read_available(&mut stream);

    for item in ["FLAGS ", "INTERNALDATE ", "RFC822.SIZE ", "ENVELOPE "] {
        assert!(
            resp.contains(item),
            "ALL expansion missing {item:?}: {resp:?}"
        );
    }
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

/// RED: `FETCH 1 UID` on a message that is not sequence 1 must still address
/// it. Here UID 1 == seq 1, so this passes; kept as a guard for the
/// out-of-range case below.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_out_of_range_sequence_returns_no_untagged() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    // Only one message exists; sequence 2 is out of range and RFC 9051
    // §6.4.8 says the server returns tagged OK with no untagged FETCH.
    stream.write_all(b"a4 FETCH 2 UID\r\n").unwrap();
    let resp = read_available(&mut stream);

    assert!(
        !resp.contains("* 2 FETCH"),
        "out-of-range sequence must not produce an untagged FETCH: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_body_peek_header_returns_headers() {
    let addr = start_server().await;
    let mut stream = login_with_message(*addr, MESSAGE);

    stream
        .write_all(b"a4 FETCH 1 BODY.PEEK[HEADER]\r\n")
        .unwrap();
    let resp = read_available(&mut stream);

    // `.PEEK` is only a request modifier; the response item is plain `BODY[HEADER]`.
    assert!(
        resp.contains("BODY[HEADER] {"),
        "expected a `BODY[HEADER] {{n}}` literal: {resp:?}"
    );
    assert!(
        resp.contains("From: alice@example.com") && resp.contains("Subject: greetings"),
        "header block missing expected fields: {resp:?}"
    );
    // `[HEADER]` stops at the blank-line separator — the body must not appear.
    assert!(
        !resp.contains("Hello, Bob."),
        "`BODY[HEADER]` must not include the message body: {resp:?}"
    );
    assert!(
        resp.contains("a4 OK") && resp.contains("FETCH completed"),
        "expected tagged OK completion: {resp:?}"
    );
}
