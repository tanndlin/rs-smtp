use sqlx::types::chrono::{DateTime, Utc};

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

impl Email {
    pub fn new(sender: String, recipients_to: Vec<String>, raw: String) -> Self {
        let recipients_cc = header(&raw, "Cc")
            .map(|cc| cc.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

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
