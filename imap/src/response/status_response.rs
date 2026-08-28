use crate::{command::StatusCommand, response::ServerResponse};
use util::EncodeTo;

#[derive(Debug)]
pub struct StatusResponse {
    request_tag: String,
    mailbox: String,
    messages: Option<u64>,
    next_uid: Option<u64>,
    validity_uid: Option<u64>,
    unseen: Option<u64>,
    deleted: Option<u64>,
}

impl StatusResponse {
    pub fn new(
        cmd: StatusCommand,
        messages: Option<u64>,
        next_uid: Option<u64>,
        validity_uid: Option<u64>,
        unseen: Option<u64>,
        deleted: Option<u64>,
    ) -> Self {
        let request_tag = cmd.tag;
        let mailbox = cmd.mailbox;
        Self {
            request_tag,
            mailbox,
            messages,
            next_uid,
            validity_uid,
            unseen,
            deleted,
        }
    }
}

impl EncodeTo for StatusResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let StatusResponse {
            request_tag,
            mailbox,
            messages,
            next_uid,
            validity_uid,
            unseen,
            deleted,
        } = self;

        buf.extend(format!("* STATUS {mailbox} (").bytes());
        if let Some(messages) = messages {
            if *buf.last().unwrap() != b'(' {
                buf.push(b'(');
            }

            buf.extend(format!("MESSAGES {messages}").bytes());
        }

        if let Some(next_uid) = next_uid {
            if *buf.last().unwrap() != b'(' {
                buf.push(b'(');
            }

            buf.extend(format!("UIDNEXT {next_uid}").bytes());
        }

        if let Some(validity_uid) = validity_uid {
            if *buf.last().unwrap() != b'(' {
                buf.push(b'(');
            }

            buf.extend(format!("UIDVAILIDITY {validity_uid}").bytes());
        }

        if let Some(unseen) = unseen {
            if *buf.last().unwrap() != b'(' {
                buf.push(b'(');
            }

            buf.extend(format!("UNSEEN {unseen}").bytes());
        }

        if let Some(deleted) = deleted {
            if *buf.last().unwrap() != b'(' {
                buf.push(b'(');
            }

            buf.extend(format!("DELETED {deleted}").bytes());
        }

        buf.extend(b")\r\n");
        buf.extend(format!("{request_tag} OK STATUS completed\r\n").bytes());
    }
}

impl From<StatusResponse> for ServerResponse {
    fn from(res: StatusResponse) -> Self {
        ServerResponse::Status(res)
    }
}
