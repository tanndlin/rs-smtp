use util::EncodeTo;

use crate::{command::LogoutCommand, response::ServerResponseTrait};

#[derive(Debug)]
pub struct LogoutResponse {
    pub request_tag: String,
}

impl ServerResponseTrait<LogoutCommand> for LogoutResponse {
    fn respond_to(cmd: LogoutCommand) -> Self {
        Self {
            request_tag: cmd.tag,
        }
    }
}

impl EncodeTo for LogoutResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        buf.extend_from_slice(b"* BYE IMAP4rev2 Server logging out\r\n");
        buf.extend_from_slice(format!("{tag} OK LOGOUT completed\r\n").as_bytes());
    }
}
