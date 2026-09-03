use crate::{command::SelectCommand, response::ServerResponse};
use util::EncodeTo;

#[derive(Debug)]
pub struct SelectResponse {
    request_tag: String,
    exists: usize,
    next_uid: u64,
    validity_uid: u64,
}

impl SelectResponse {
    pub fn new(cmd: SelectCommand, exists: usize, next_uid: u64, validity_uid: u64) -> Self {
        let request_tag = cmd.tag;
        Self {
            request_tag,
            exists,
            next_uid,
            validity_uid,
        }
    }
}

impl EncodeTo for SelectResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let SelectResponse {
            request_tag,
            exists,
            next_uid,
            validity_uid,
        } = self;

        buf.extend_from_slice(format!("* {exists} EXISTS\r\n").as_bytes());
        buf.extend_from_slice(b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)\r\n");
        buf.extend_from_slice(
            format!("* OK [UIDVALIDITY {validity_uid}] UIDs valid\r\n").as_bytes(),
        );
        buf.extend_from_slice(
            format!("* OK [UIDNEXT {next_uid}] Predicted next UID\r\n").as_bytes(),
        );
        buf.extend_from_slice(
            format!("{request_tag} OK [READ-WRITE] SELECT completed\r\n").as_bytes(),
        );
    }
}

impl From<SelectResponse> for ServerResponse {
    fn from(res: SelectResponse) -> Self {
        ServerResponse::Select(res)
    }
}
