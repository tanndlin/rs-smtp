use util::EncodeTo;

use crate::{errors::CommandParseError, response::ServerResponse};

#[derive(Debug)]
pub struct ServerErrorResponse {
    pub tag: Option<String>,
    pub reason: ServerErrorReason,
}

#[derive(Debug)]
pub enum ServerErrorReason {
    CommandParseError(CommandParseError),
    ProtocolViolation(String),
    Deny(String),
}

impl ServerErrorResponse {
    fn tag_str(&self) -> String {
        self.tag.to_owned().unwrap_or("*".to_string())
    }
}

impl EncodeTo for ServerErrorResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.tag_str();
        match self.reason {
            ServerErrorReason::CommandParseError(_) => todo!(),
            ServerErrorReason::ProtocolViolation(reason) => {
                buf.extend(format!("{tag} BAD {reason}\r\n").bytes())
            }
            ServerErrorReason::Deny(reason) => buf.extend(format!("{tag} NO {reason}\r\n").bytes()),
        }
    }
}

impl From<ServerErrorResponse> for ServerResponse {
    fn from(res: ServerErrorResponse) -> Self {
        ServerResponse::Error(res)
    }
}
