use chrono::{DateTime, Utc};

pub struct Email {
    pub sender: String,
    pub recipients_to: Vec<String>,
    pub recipients_cc: Vec<String>,
    pub subject: Option<String>,
    pub sent_date: DateTime<Utc>,
    pub body: String,
    pub raw: String,
    pub message_id: Option<String>,
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

impl Email {
    /// Build a message when the envelope sender and recipients are known
    /// out-of-band (the SMTP ingest path supplies them from `MAIL FROM` /
    /// `RCPT TO`).
    pub fn new(sender: String, recipients_to: Vec<String>, raw: String) -> Self {
        Self::build(sender, recipients_to, raw)
    }

    /// Build a message that arrived with no envelope (IMAP `APPEND`): the
    /// sender and recipients are taken from the `From:` and `To:` headers.
    pub fn from_raw(raw: String) -> Self {
        let sender = header(&raw, "From").unwrap_or_default().to_string();
        let recipients_to = address_list(&raw, "To");
        Self::build(sender, recipients_to, raw)
    }

    fn build(sender: String, recipients_to: Vec<String>, raw: String) -> Self {
        let recipients_cc = address_list(&raw, "Cc");

        let subject = header(&raw, "Subject").map(|s| s.to_string());

        let sent_date = header(&raw, "Date")
            .map(DateTime::parse_from_rfc2822)
            .and_then(Result::ok)
            .map(Into::into)
            .unwrap_or_else(Utc::now);

        let body = raw
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();

        let message_id = header(&raw, "Message-ID")
            .map(|id| id.trim_start_matches('<').trim_end_matches('>').to_string());

        Email {
            sender,
            recipients_to,
            recipients_cc,
            subject,
            sent_date,
            body,
            raw,
            message_id,
        }
    }
}
