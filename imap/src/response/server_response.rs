use crate::response::{
    AppendOkResponse, CapabilityResponse, CheckResponse, ContinuationResponse, CreateResponse,
    FetchResponse, ListResponse, LoginResponse, LogoutResponse, LsubResponse, NoopResponse,
    SelectResponse, ServerErrorResponse, StatusResponse,
};
use util::EncodeTo;

#[derive(Debug)]
pub enum ServerResponse {
    Capability(CapabilityResponse),
    Login(LoginResponse),
    List(ListResponse),
    Lsub(LsubResponse),
    Error(ServerErrorResponse),
    Select(SelectResponse),
    Status(StatusResponse),
    Continue(ContinuationResponse),
    Logout(LogoutResponse),
    Append(AppendOkResponse),
    Fetch(FetchResponse),
    Create(CreateResponse),
    Noop(NoopResponse),
    Check(CheckResponse),
}

impl EncodeTo for ServerResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            ServerResponse::Capability(res) => res.encode_to(buf),
            ServerResponse::Login(res) => res.encode_to(buf),
            ServerResponse::Error(res) => res.encode_to(buf),
            ServerResponse::List(res) => res.encode_to(buf),
            ServerResponse::Lsub(res) => res.encode_to(buf),
            ServerResponse::Select(res) => res.encode_to(buf),
            ServerResponse::Status(res) => res.encode_to(buf),
            ServerResponse::Continue(res) => res.encode_to(buf),
            ServerResponse::Logout(res) => res.encode_to(buf),
            ServerResponse::Append(res) => res.encode_to(buf),
            ServerResponse::Fetch(res) => res.encode_to(buf),
            ServerResponse::Create(res) => res.encode_to(buf),
            ServerResponse::Noop(res) => res.encode_to(buf),
            ServerResponse::Check(res) => res.encode_to(buf),
        }
    }
}

pub trait ServerResponseTrait<T>: EncodeTo {
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
