use crate::command::StatusCommand;
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

        let attrs = [
            ("MESSAGES", messages),
            ("UIDNEXT", next_uid),
            ("UIDVALIDITY", validity_uid),
            ("UNSEEN", unseen),
            ("DELETED", deleted),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.map(|v| format!("{name} {v}")))
        .collect::<Vec<_>>()
        .join(" ");

        buf.extend_from_slice(
            format!("* STATUS {mailbox} ({attrs})\r\n{request_tag} OK STATUS completed\r\n")
                .as_bytes(),
        );
    }
}
