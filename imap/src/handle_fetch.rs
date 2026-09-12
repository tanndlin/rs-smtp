use std::sync::Arc;

use sqlx::{Pool, Postgres};
use util::Email;

use crate::command::{BodyFetchable, Fetchable, Partial, Section, SectionText};

/// Render one FETCH data item as the string to splice into the response.
pub async fn get_fetchable(
    db_pool: Arc<Pool<Postgres>>,
    message_id: u64,
    fetchable: &Fetchable,
) -> String {
    let id = message_id as i32;

    // UID is storage bookkeeping, not part of the message - look it up directly.
    if let Fetchable::UID = fetchable {
        return get_uid(&db_pool, id).await;
    }

    let email = Email::fetch(&*db_pool, id)
        .await
        .expect("failed to load message for FETCH");

    match fetchable {
        Fetchable::Envelope => envelope(&email),
        Fetchable::RFC822Size => email.raw_eml.len().to_string(),
        Fetchable::All => todo!(),
        Fetchable::Fast => todo!(),
        Fetchable::Full => todo!(),
        Fetchable::Binary(_) => todo!(),
        Fetchable::Body(b) => handle_body(email, b),
        Fetchable::BodyStructure => todo!(),
        Fetchable::Flags => format!("({})", email.flags.join(" ")),
        Fetchable::Internaldate => internal_date(&email),
        Fetchable::UID => unreachable!("handled above"),
    }
}

fn handle_body(email: Email, b: &BodyFetchable) -> String {
    let (section, partial) = match b {
        BodyFetchable::Full => return body_structure(&email),
        BodyFetchable::Section {
            section, partial, ..
        } => (section, partial),
    };

    let (key, data) = match section {
        Section::Full => (String::new(), email.raw_eml.clone()),
        Section::Msg(text) => msg_section(&email, text),
        Section::Part { part, text } => part_section(&email, part, text),
    };

    literal(&key, &data, partial)
}

/// Format a `BODY[<key>]` literal response item, applying `<start.count>`
/// truncation when a partial was requested.
fn literal(key: &str, data: &str, partial: &Option<Partial>) -> String {
    match partial {
        None => format!("BODY[{key}] {{{}}}\r\n{data}", data.len()),
        Some(Partial { start, count }) => {
            let start = (*start as usize).min(data.len());
            let end = start.saturating_add(*count as usize).min(data.len());
            let slice = &data[start..end];
            format!("BODY[{key}]<{start}> {{{}}}\r\n{slice}", slice.len())
        }
    }
}

fn msg_section(email: &Email, text: &SectionText) -> (String, String) {
    match text {
        SectionText::Header => ("HEADER".into(), email.get_headers()),
        SectionText::HeaderFields(names) => (
            format!("HEADER.FIELDS ({})", names.join(" ")),
            select_headers(email, names, true),
        ),
        SectionText::HeaderFieldsNot(names) => (
            format!("HEADER.FIELDS.NOT ({})", names.join(" ")),
            select_headers(email, names, false),
        ),
        SectionText::Text => ("TEXT".into(), body_octets(&email.raw_eml).to_string()),
        SectionText::Mime => todo!(),
    }
}

fn part_section(email: &Email, part: &[u32], text: &Option<SectionText>) -> (String, String) {
    match (part, text) {
        ([1], None) => ("1".into(), body_octets(&email.raw_eml).to_string()),
        _ => todo!(),
    }
}

/// The header lines whose field name matches one of `names` (`keep`) or none
/// of them (`!keep`), rejoined with CRLF.
fn select_headers(email: &Email, names: &[String], keep: bool) -> String {
    email
        .get_headers()
        .split("\r\n")
        .filter(|line| names.iter().any(|n| line.to_uppercase().starts_with(n)) == keep)
        .collect::<Vec<_>>()
        .join("\r\n")
}

fn body_structure(email: &Email) -> String {
    let body = body_octets(&email.raw_eml);
    format!(
        "BODY (\"TEXT\" \"PLAIN\" (\"CHARSET\" \"US-ASCII\") NIL NIL \"7BIT\" {} {})",
        body.len(),
        body.matches("\r\n").count()
    )
}

fn body_octets(raw: &str) -> &str {
    raw.split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("")
}

/// The message's INTERNALDATE as an IMAP `date-time` quoted string, e.g.
/// `"23-Oct-2024 19:00:00 +0000"`. The day is space-padded per RFC 9051.
fn internal_date(email: &Email) -> String {
    format!("\"{}\"", email.received_at.format("%e-%b-%Y %H:%M:%S %z"))
}

fn envelope(email: &Email) -> String {
    // Sender / Reply-To default to From when the message carried no such header.
    let from = addrs_from_str(&email.from);
    let sender = email
        .sender
        .as_deref()
        .map(addrs_from_str)
        .unwrap_or_else(|| from.clone());
    let reply_to = email
        .reply_to
        .as_deref()
        .map(addrs_from_str)
        .unwrap_or_else(|| from.clone());

    let date = email
        .sent_date
        .map(|d| quote(&d.to_rfc2822()))
        .unwrap_or_else(nil);
    let subject = nstring(email.subject.as_deref());
    let in_reply_to = bracketed_id(email.in_reply_to.as_deref());
    let message_id = bracketed_id(email.message_id.as_deref());

    let fields = [
        date,
        subject,
        from,
        sender,
        reply_to,
        addrs_from_vec(&email.recipients_to),
        addrs_from_vec(&email.recipients_cc),
        addrs_from_vec(&email.recipients_bcc),
        in_reply_to,
        message_id,
    ];

    format!("({})", fields.join(" "))
}

/// An IMAP `nstring`: a quoted string, or `NIL` when the value is absent.
fn nstring(value: Option<&str>) -> String {
    match value {
        Some(v) => quote(v),
        None => nil(),
    }
}

/// A stored message-id (angle brackets stripped) rendered as the envelope
/// wants it: `"<id>"`, or `NIL`.
fn bracketed_id(id: Option<&str>) -> String {
    match id {
        Some(id) => quote(&format!("<{id}>")),
        None => nil(),
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn nil() -> String {
    "NIL".to_string()
}

fn addrs_from_str(raw: &str) -> String {
    render_addrs(std::iter::once(raw))
}

fn addrs_from_vec(entries: &[String]) -> String {
    render_addrs(entries.iter().map(String::as_str))
}

/// Render address entries as an IMAP address list `((name adl mailbox host) ...)`,
/// or `NIL` when there are none. Each entry may itself be comma-separated.
fn render_addrs<'a>(entries: impl Iterator<Item = &'a str>) -> String {
    let rendered: String = entries
        .flat_map(|entry| entry.split(','))
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(address)
        .collect();

    if rendered.is_empty() {
        nil()
    } else {
        format!("({rendered})")
    }
}

/// Render one `name <mailbox@host>` / `mailbox@host` entry as an IMAP
/// `address` structure. `adl` (source-route) is always `NIL`.
fn address(entry: &str) -> String {
    let (name, addr) = match entry.rfind('<') {
        Some(start) if entry.ends_with('>') => {
            let name = entry[..start].trim().trim_matches('"');
            let name = if name.is_empty() { None } else { Some(name) };
            (name, &entry[start + 1..entry.len() - 1])
        }
        _ => (None, entry),
    };

    let (mailbox, host) = addr.split_once('@').unwrap_or((addr, ""));
    let host = if host.is_empty() { nil() } else { quote(host) };

    format!("({} NIL {} {host})", nstring(name), quote(mailbox))
}

async fn get_uid(db_pool: &Pool<Postgres>, id: i32) -> String {
    sqlx::query!("SELECT uid FROM mail WHERE id = $1", id)
        .fetch_one(db_pool)
        .await
        .unwrap() // TODO: Check for 404
        .uid
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Partial;
    use sqlx::types::chrono::{DateTime, Utc};

    /// Fixed INTERNALDATE for deterministic tests: 23 Oct 2024 19:00:00 UTC.
    fn received_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-10-23T19:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// A minimal RFC 5322 message: five header lines, the blank-line
    /// separator, and one line of body.
    const RAW: &str = "Date: Wed, 23 Oct 2024 19:00:00 +0000\r\n\
From: alice@example.com\r\n\
To: bob@example.com\r\n\
Subject: greetings\r\n\
Message-ID: <abc123@example.com>\r\n\
\r\n\
Hello, Bob.\r\n";

    /// The body octets that follow the header separator in `RAW`.
    const BODY: &str = "Hello, Bob.\r\n";

    fn sample_email() -> Email {
        Email::from_raw(received_at(), RAW.to_string())
    }

    fn section(section: Section) -> BodyFetchable {
        BodyFetchable::Section {
            peek: false,
            section,
            partial: None,
        }
    }

    // ------------------------- envelope (passing) -------------------------

    /// `address` keeps a quoted display name and renders each comma-separated
    /// recipient as its own `address` structure.
    #[test]
    fn envelope_renders_display_name_and_multiple_recipients() {
        let raw = "Date: Thu, 24 Oct 2024 08:15:00 +0000\r\n\
From: \"Alice Example\" <alice@example.com>\r\n\
To: bob@example.com, carol@example.net\r\n\
Subject: hi\r\n\
Message-ID: <m1@example.com>\r\n\
\r\n\
body\r\n";
        let env = envelope(&Email::from_raw(received_at(), raw.to_string()));

        assert!(
            env.contains("((\"Alice Example\" NIL \"alice\" \"example.com\"))"),
            "from address structure wrong: {env}"
        );
        assert!(
            env.contains("((NIL NIL \"bob\" \"example.com\")(NIL NIL \"carol\" \"example.net\"))"),
            "to address list wrong: {env}"
        );
    }

    /// INTERNALDATE renders as a quoted IMAP `date-time`, taken from
    /// `received_at` and not the `Date:` header.
    #[test]
    fn internal_date_renders_as_quoted_imap_date_time() {
        assert_eq!(
            internal_date(&sample_email()),
            "\"23-Oct-2024 19:00:00 +0000\""
        );
    }

    // --------------------------- body (RED) ------------------------------
    //
    // Every test below drives a `BODY[...]` section that `handle_body` still
    // answers with `todo!()`. They pin the RFC 9051 §7.5.2 response item and
    // are expected to fail until each arm is implemented.

    /// RED: `BODY[TEXT]` returns only the body, keyed `BODY[TEXT]`.
    #[test]
    fn body_text_returns_body_only() {
        let out = handle_body(sample_email(), &section(Section::Msg(SectionText::Text)));
        assert_eq!(out, format!("BODY[TEXT] {{{}}}\r\n{BODY}", BODY.len()));
    }

    /// RED: `BODY[HEADER.FIELDS (SUBJECT)]` returns just the named header
    /// line(s) plus the terminating CRLF, and nothing else.
    #[test]
    fn body_header_fields_returns_named_headers_only() {
        let out = handle_body(
            sample_email(),
            &section(Section::Msg(SectionText::HeaderFields(vec![
                "SUBJECT".into(),
            ]))),
        );
        assert!(
            out.starts_with("BODY[HEADER.FIELDS (SUBJECT)] {"),
            "wrong response item key: {out:?}"
        );
        assert!(
            out.contains("Subject: greetings"),
            "missing Subject: {out:?}"
        );
        assert!(
            !out.contains("From: alice@example.com"),
            "must not include unlisted headers: {out:?}"
        );
    }

    #[test]
    fn body_header_fields_not_excludes_named_headers() {
        let out = handle_body(
            sample_email(),
            &section(Section::Msg(SectionText::HeaderFieldsNot(vec![
                "SUBJECT".into(),
            ]))),
        );
        assert!(
            out.starts_with("BODY[HEADER.FIELDS.NOT (SUBJECT)] {"),
            "wrong response item key: {out:?}"
        );
        assert!(
            out.contains("From: alice@example.com"),
            "missing an un-excluded header: {out:?}"
        );
        assert!(
            !out.contains("Subject: greetings"),
            "excluded header still present: {out:?}"
        );
    }

    /// RED: for a non-multipart message, `BODY[1]` is the message body.
    #[test]
    fn body_numbered_part_returns_part_payload() {
        let out = handle_body(
            sample_email(),
            &section(Section::Part {
                part: vec![1],
                text: None,
            }),
        );
        assert_eq!(out, format!("BODY[1] {{{}}}\r\n{BODY}", BODY.len()));
    }

    /// RED: bare `BODY` renders the non-extensible BODYSTRUCTURE, a
    /// parenthesised structure keyed simply `BODY`.
    #[test]
    fn bare_body_renders_body_structure() {
        let out = handle_body(sample_email(), &BodyFetchable::Full);
        assert!(
            out.starts_with("BODY (") || out.starts_with("BODY("),
            "expected a parenthesised body structure: {out:?}"
        );
    }

    /// RED: `BODY[]<0.5>` returns only the first five octets and the response
    /// item carries the origin offset: `BODY[]<0> {5}`.
    #[test]
    fn body_full_with_partial_truncates_to_range() {
        let out = handle_body(
            sample_email(),
            &BodyFetchable::Section {
                peek: false,
                section: Section::Full,
                partial: Some(Partial { start: 0, count: 5 }),
            },
        );
        assert_eq!(out, format!("BODY[]<0> {{5}}\r\n{}", &RAW[..5]));
    }
}
