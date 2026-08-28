use util::EncodeTo;

use crate::{
    command::LogoutCommand,
    response::{ServerResponse, ServerResponseTrait},
    server_response_from_impl,
};

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

server_response_from_impl!(LogoutResponse, Logout);

impl EncodeTo for LogoutResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        buf.extend(b"* BYE IMAP4rev1 Server logging out\r\n");
        buf.extend(format!("{tag} OK LOGOUT completed\r\n").bytes());
    }
}
