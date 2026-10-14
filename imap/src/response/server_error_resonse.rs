use util::EncodeTo;

use crate::errors::CommandParseError;

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
    pub fn protocol_violation(tag: String, reason: String) -> Self {
        Self {
            tag: Some(tag),
            reason: ServerErrorReason::ProtocolViolation(reason),
        }
    }

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
                buf.extend_from_slice(format!("{tag} BAD {reason}\r\n").as_bytes())
            }
            ServerErrorReason::Deny(reason) => {
                buf.extend_from_slice(format!("{tag} NO {reason}\r\n").as_bytes())
            }
        }
    }
}
