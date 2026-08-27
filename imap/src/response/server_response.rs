use tokio::net::windows::named_pipe::PipeEnd::Server;

use crate::response::CapabilityResponse;
use util::EncodeTo;

pub enum ServerResponse {
    Capability(CapabilityResponse),
}

impl ServerResponse {
    pub fn new_capability() -> ServerResponse {
        CapabilityResponse::new().into()
    }
}

impl EncodeTo for ServerResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            ServerResponse::Capability(res) => res.encode_to(buf),
        }
    }
}
