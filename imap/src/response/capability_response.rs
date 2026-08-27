use crate::{response::ServerResponse, util::EncodeTo};

pub struct CapabilityResponse {}

impl CapabilityResponse {
    pub fn new() -> Self {
        Self {}
    }
}

impl EncodeTo for CapabilityResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend(b". CAPABILITY IMAP4rev2\r\n");
    }
}

impl From<CapabilityResponse> for ServerResponse {
    fn from(res: CapabilityResponse) -> Self {
        ServerResponse::Capability(res)
    }
}
