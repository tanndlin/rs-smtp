use crate::{
    command::CapabilityCommand,
    response::{ServerResponse, server_response::ServerResponseTrait},
};
use util::EncodeTo;

#[derive(Debug)]
pub struct CapabilityResponse {
    request_tag: String,
}

impl ServerResponseTrait<CapabilityCommand> for CapabilityResponse {
    fn respond_to(cmd: CapabilityCommand) -> Self {
        let request_tag = cmd.tag;
        Self { request_tag }
    }
}

impl EncodeTo for CapabilityResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let tag = self.request_tag;

        buf.extend_from_slice(
            format!("* CAPABILITY IMAP4rev2\r\n{tag} OK CAPABILITY completed\r\n").as_bytes(),
        );
    }
}

impl From<CapabilityResponse> for ServerResponse {
    fn from(res: CapabilityResponse) -> Self {
        ServerResponse::Capability(res)
    }
}
