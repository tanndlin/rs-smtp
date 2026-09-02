use std::sync::Arc;

use sqlx::{Pool, Postgres};
use util::Email;

use crate::command::Fetchable;

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
        Fetchable::Body(_) => todo!(),
        Fetchable::BodyStructure => todo!(),
        Fetchable::Flags => todo!(),
        Fetchable::Internaldate => todo!(),
        Fetchable::UID => unreachable!("handled above"),
    }
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
