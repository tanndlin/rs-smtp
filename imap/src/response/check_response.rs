use util::EncodeTo;

use crate::{command::CheckCommand, response::ServerResponseTrait};

#[derive(Debug)]
pub struct CheckResponse {
    request_tag: String,
}

impl ServerResponseTrait<CheckCommand> for CheckResponse {
    fn respond_to(cmd: CheckCommand) -> Self {
        CheckResponse {
            request_tag: cmd.tag,
        }
    }
}

impl EncodeTo for CheckResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.request_tag.as_bytes());
        buf.extend_from_slice(b" OK CHECK completed\r\n");
    }
}
