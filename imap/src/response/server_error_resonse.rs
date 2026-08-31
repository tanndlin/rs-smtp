use util::EncodeTo;

use crate::errors::CommandParseError;

#[derive(Debug)]
pub enum ServerErrorResponse {
    CommandParseError(CommandParseError),
    ProtocolViolation(String),
}

impl EncodeTo for ServerErrorResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            ServerErrorResponse::CommandParseError(_) => todo!(),
            ServerErrorResponse::ProtocolViolation(reason) => {
                buf.extend(format!("* BAD {reason}").bytes())
            }
        }
    }
}
