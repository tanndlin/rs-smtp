use sqlx::types::chrono::{DateTime, Utc};

pub struct Email {
    pub sender: String,
    pub recipients_to: Vec<String>,
    pub recipients_cc: Vec<String>,
    pub subject: Option<String>,
    pub sent_date: DateTime<Utc>,
    pub body: String,
    pub raw: String,
}

impl Email {
    pub fn new(sender: String, recipients_to: Vec<String>, raw: String) -> Self {
        let mut lines = raw.split("\r\n");

        let cc_line = lines
            .find(|l| l.starts_with("Cc:"))
            .unwrap_or("")
            .trim_start_matches("Cc:");
        let recipients_cc = cc_line.split(",").map(|s| s.trim().to_string()).collect();

        let subject_line = lines
            .find(|l| l.starts_with("Subject:"))
            .map(|s| s.trim_start_matches("Subject:"));
        let subject = subject_line.map(|s| s.trim().to_string());

        let date_line = lines
            .find(|l| l.starts_with("Date:"))
            .map(|l| l.trim_start_matches("Date:"));
        let sent_date = date_line
            .map(|d| d.trim())
            .map(DateTime::parse_from_rfc2822)
            .unwrap_or_else(|| Ok(Utc::now().into()))
            .unwrap_or_else(|_| Utc::now().into());

        let body = raw
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();

        Email {
            sender,
            recipients_to,
            recipients_cc,
            subject,
            sent_date: sent_date.into(),
            body,
            raw,
        }
    }
}
