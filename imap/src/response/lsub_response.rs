use util::EncodeTo;

use crate::{
    response::{MailboxListEntry, ServerResponse},
    server_response_from_impl,
};

#[derive(Debug)]
pub struct LsubResponse {
    request_tag: String,
    mailboxes: Vec<MailboxListEntry>,
}

impl LsubResponse {
    pub fn new(request_tag: String, mailboxes: Vec<MailboxListEntry>) -> Self {
        Self {
            request_tag,
            mailboxes,
        }
    }
}

impl EncodeTo for LsubResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        for mbox in &self.mailboxes {
            let attrs = mbox.attributes.join(" ");
            let delim = mbox
                .delimiter
                .map(|c| format!("\"{c}\""))
                .unwrap_or_else(|| "NIL".to_string());
            buf.extend_from_slice(format!("* LSUB ({attrs}) {delim} {}\r\n", mbox.name).as_bytes());
        }
        buf.extend_from_slice(format!("{tag} OK LSUB completed\r\n").as_bytes());
    }
}

server_response_from_impl!(LsubResponse, Lsub);
