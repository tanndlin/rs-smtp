use util::EncodeTo;

use crate::{
    command::NoopCommand,
    response::{ServerResponse, ServerResponseTrait},
    server_response_from_impl,
};

#[derive(Debug)]
pub struct NoopResponse {
    request_tag: String,
}

impl ServerResponseTrait<NoopCommand> for NoopResponse {
    fn respond_to(cmd: NoopCommand) -> Self {
        NoopResponse {
            request_tag: cmd.tag,
        }
    }
}

server_response_from_impl!(NoopResponse, Noop);

impl EncodeTo for NoopResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.request_tag.as_bytes());
        buf.extend_from_slice(b" OK NOOP completed\r\n");
    }
}
