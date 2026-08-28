use util::EncodeTo;

use crate::{response::ServerResponse, server_response_from_impl};

#[derive(Debug)]
pub struct ListResponse {
    request_tag: String,
    mailboxes: Vec<MailboxListEntry>,
}

impl ListResponse {
    pub fn new(request_tag: String, mailboxes: Vec<MailboxListEntry>) -> Self {
        Self {
            request_tag,
            mailboxes,
        }
    }
}

#[derive(Debug)]
pub struct MailboxListEntry {
    attributes: Vec<&'static str>, // e.g. ["\\Unmarked", "\\HasNoChildren"]
    delimiter: Option<char>,
    name: String,
}

impl MailboxListEntry {
    pub fn new(attributes: Vec<&'static str>, delimiter: Option<char>, name: String) -> Self {
        Self {
            attributes,
            delimiter,
            name,
        }
    }
}

impl EncodeTo for ListResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        for mbox in &self.mailboxes {
            let attrs = mbox.attributes.join(" ");
            let delim = mbox
                .delimiter
                .map(|c| format!("\"{c}\""))
                .unwrap_or_else(|| "NIL".to_string());
            buf.extend(format!("* LIST ({attrs}) {delim} {}\r\n", mbox.name).bytes());
        }
        buf.extend(format!("{tag} OK LIST completed\r\n").bytes());
    }
}

server_response_from_impl!(ListResponse, List);
