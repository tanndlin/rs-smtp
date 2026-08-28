use crate::response::{CapabilityResponse, LoginResponse};
use util::EncodeTo;

#[derive(Debug)]
pub enum ServerResponse {
    Capability(CapabilityResponse),
    Login(LoginResponse),
}

impl EncodeTo for ServerResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            ServerResponse::Capability(res) => res.encode_to(buf),
            ServerResponse::Login(res) => res.encode_to(buf),
        }
    }
}

pub trait ServerResponseTrait<T> {
    fn respond_to(cmd: T) -> Self;
}

#[macro_export]
macro_rules! server_response_from_impl {
    ($type: tt,$variant: ident) => {
        impl From<$type> for ServerResponse {
            fn from(cmd: $type) -> Self {
                ServerResponse::$variant(cmd)
            }
        }
    };
}
