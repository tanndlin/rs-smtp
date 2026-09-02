use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Email {
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub from: String,
    pub sender: Option<String>,
    pub reply_to: Option<String>,
    pub recipients_to: Vec<String>,
    pub recipients_cc: Vec<String>,
    pub recipients_bcc: Vec<String>,
    pub subject: Option<String>,
    pub sent_date: Option<DateTime<Utc>>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub raw_eml: String,
}

fn header<'a>(raw: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}:").to_ascii_lowercase();
    raw.split("\r\n")
        .take_while(|l| !l.is_empty())
        .find(|l| l.to_ascii_lowercase().starts_with(&prefix))
        .map(|l| l[prefix.len()..].trim())
}

fn address_list(raw: &str, name: &str) -> Vec<String> {
    header(raw, name)
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

/// Reduce an address to its comparable form: the `addr-spec` inside any
/// `Display Name <...>` wrapper, lowercased.
fn addr_only(addr: &str) -> String {
    let inner = match (addr.find('<'), addr.find('>')) {
        (Some(open), Some(close)) if open < close => &addr[open + 1..close],
        _ => addr,
    };
    inner.trim().to_ascii_lowercase()
}

fn unbracket(id: &str) -> String {
    id.trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

impl Email {
    /// Build a message when the envelope recipients are known out-of-band
    /// (the SMTP ingest path supplies them from `RCPT TO`). The envelope
    /// sender comes from `MAIL FROM`.
    pub fn new(from: String, envelope: Vec<String>, raw: String) -> Self {
        Self::build(from, Some(envelope), raw)
    }

    /// Build a message that arrived with no envelope (IMAP `APPEND`): the
    /// sender and recipients are taken entirely from the headers.
    pub fn from_raw(raw: String) -> Self {
        let from = header(&raw, "From").unwrap_or_default().to_string();
        Self::build(from, None, raw)
    }

    /// `envelope` is the `RCPT TO` list when known (SMTP), or `None` when the
    /// message arrived header-only (IMAP `APPEND`). `To:` / `Cc:` always come
    /// from the headers; BCC recipients are the envelope entries not openly
    /// addressed there, or - with no envelope - an explicit `Bcc:` header.
    pub fn build(from: String, envelope: Option<Vec<String>>, raw: String) -> Self {
        let recipients_to = address_list(&raw, "To");
        let recipients_cc = address_list(&raw, "Cc");

        let recipients_bcc: Vec<String> = match envelope {
            Some(envelope) => {
                let openly_addressed: Vec<String> = recipients_to
                    .iter()
                    .chain(&recipients_cc)
                    .map(|a| addr_only(a))
                    .collect();
                envelope
                    .into_iter()
                    .filter(|a| !openly_addressed.contains(&addr_only(a)))
                    .collect()
            }
            None => address_list(&raw, "Bcc"),
        };

        let sender = header(&raw, "Sender").map(str::to_string);
        let reply_to = header(&raw, "Reply-To").map(str::to_string);
        let subject = header(&raw, "Subject").map(str::to_string);
        let message_id = header(&raw, "Message-ID").map(unbracket);
        let in_reply_to = header(&raw, "In-Reply-To").map(unbracket);
        let sent_date = header(&raw, "Date")
            .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
            .map(Into::into);

        let body_text = raw.split_once("\r\n\r\n").map(|(_, b)| b.to_string());

        Email {
            message_id,
            in_reply_to,
            from,
            sender,
            reply_to,
            recipients_to,
            recipients_cc,
            recipients_bcc,
            subject,
            sent_date,
            body_text,
            body_html: None,
            raw_eml: raw,
        }
    }

    /// Persist this message into `mailbox`, assigning it `uid`.
    pub async fn insert<'e, E>(self, executor: E, mailbox: &str, uid: i32) -> sqlx::Result<()>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let Email {
            message_id,
            in_reply_to,
            from,
            sender,
            reply_to,
            recipients_to,
            recipients_cc,
            recipients_bcc,
            subject,
            sent_date,
            body_text,
            body_html,
            raw_eml,
        } = self;

        sqlx::query!(
            r#"INSERT INTO mail
                 (mailbox_id, uid, message_id, in_reply_to, "from", sender, reply_to,
                  recipients_to, recipients_cc, recipients_bcc,
                  subject, sent_date, body_text, body_html, raw_eml)
               VALUES
                 ((SELECT id FROM mailboxes WHERE name = $1),
                  $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"#,
            mailbox,
            uid,
            message_id,
            in_reply_to,
            from,
            sender,
            reply_to,
            &recipients_to,
            &recipients_cc,
            &recipients_bcc,
            subject,
            sent_date,
            body_text,
            body_html,
            raw_eml,
        )
        .execute(executor)
        .await
        .map(|_| ())
    }

    /// Load the message stored at `mail.id = id`.
    pub async fn fetch<'e, E>(executor: E, id: i32) -> sqlx::Result<Self>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as!(
            Email,
            r#"SELECT message_id, in_reply_to, "from", sender, reply_to,
                      recipients_to, recipients_cc, recipients_bcc,
                      subject, sent_date, body_text, body_html, raw_eml
               FROM mail WHERE id = $1"#,
            id,
        )
        .fetch_one(executor)
        .await
    }
}
