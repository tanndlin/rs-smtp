use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::command::Fetchable;

// Returns the fetchable result as a string to encode to response
pub async fn get_fetchable(
    db_pool: Arc<Pool<Postgres>>,
    message_id: u64,
    fetchable: &Fetchable,
) -> String {
    match fetchable {
        Fetchable::All => todo!(),
        Fetchable::Fast => todo!(),
        Fetchable::Full => todo!(),
        Fetchable::Binary(binary_fetchable) => todo!(),
        Fetchable::Body(body_fetchable) => todo!(),
        Fetchable::BodyStructure => todo!(),
        Fetchable::Envelope => get_envelope(db_pool, message_id).await,
        Fetchable::Flags => todo!(),
        Fetchable::Internaldate => todo!(),
        Fetchable::RFC822Size => get_rfc822_size(db_pool, message_id).await,
        Fetchable::UID => get_uid(db_pool, message_id).await,
    }
}

async fn get_envelope(db_pool: Arc<Pool<Postgres>>, message_id: u64) -> String {
    let raw = sqlx::query!("SELECT raw_eml from mail WHERE id = $1", message_id as i32)
        .fetch_one(&*db_pool)
        .await
        .unwrap()
        .raw_eml
        .unwrap();

    let headers = raw.split("\r\n\r\n").next().unwrap_or(&raw);
    let from = address_list(headers, "From");
    // Sender / Reply-To default to From when the message has no header of
    // their own (RFC 9051 §7.5.2).
    let sender = address_list(headers, "Sender").or_else(|| from.clone());
    let reply_to = address_list(headers, "Reply-To").or_else(|| from.clone());

    let fields = [
        nstring(header(headers, "Date")),
        nstring(header(headers, "Subject")),
        from.unwrap_or_else(nil),
        sender.unwrap_or_else(nil),
        reply_to.unwrap_or_else(nil),
        address_list(headers, "To").unwrap_or_else(nil),
        address_list(headers, "Cc").unwrap_or_else(nil),
        address_list(headers, "Bcc").unwrap_or_else(nil),
        nstring(header(headers, "In-Reply-To")),
        nstring(header(headers, "Message-ID")),
    ];

    format!("({})", fields.join(" "))
}

/// Look up a header's raw value (trimmed, `\r\n`-folded lines not handled -
/// none of the messages we build test with need it) within a header block.
fn header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}:").to_ascii_lowercase();
    headers
        .split("\r\n")
        .find(|line| line.to_ascii_lowercase().starts_with(&prefix))
        .map(|line| line[prefix.len()..].trim())
}

/// An IMAP `nstring`: a quoted string, or `NIL` when the header is absent.
fn nstring(value: Option<&str>) -> String {
    match value {
        Some(v) => quote(v),
        None => nil(),
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn nil() -> String {
    "NIL".to_string()
}

/// Render a comma-separated address header as an IMAP address list:
/// `((name adl mailbox host) ...)`, or `None` when the header is absent.
fn address_list(headers: &str, name: &str) -> Option<String> {
    let raw = header(headers, name)?;
    let addresses: String = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(address)
        .collect();

    if addresses.is_empty() {
        None
    } else {
        Some(format!("({addresses})"))
    }
}

/// Render one `name <mailbox@host>` / `mailbox@host` entry as an IMAP
/// `address` structure. `adl` (source-route) is always `NIL` - nothing here
/// produces or consumes source-routed addresses.
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

// TODO: Fetch the mail once and pass through
async fn get_uid(db_pool: Arc<Pool<Postgres>>, message_id: u64) -> String {
    let uid = sqlx::query!("SELECT uid from mail WHERE id = $1", message_id as i32)
        .fetch_one(&*db_pool)
        .await
        .unwrap()
        .uid; // TODO: Check for 404

    uid.to_string()
}

async fn get_rfc822_size(db_pool: Arc<Pool<Postgres>>, message_id: u64) -> String {
    let raw = sqlx::query!("SELECT raw_eml from mail WHERE id = $1", message_id as i32)
        .fetch_one(&*db_pool)
        .await
        .unwrap()
        .raw_eml
        .unwrap();

    raw.len().to_string()
}
