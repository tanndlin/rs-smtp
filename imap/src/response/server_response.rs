use crate::response::CapabilityResponse;
use util::EncodeTo;

pub enum ServerResponse {
    Capability(CapabilityResponse),
}

impl EncodeTo for ServerResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            ServerResponse::Capability(res) => res.encode_to(buf),
        }
    }
}

pub trait ServerResponseTrait<T> {
    fn respond_to(cmd: T) -> Self;
}
