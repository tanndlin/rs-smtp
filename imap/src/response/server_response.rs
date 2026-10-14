use crate::response::{
    AppendOkResponse, CapabilityResponse, CheckResponse, ContinuationResponse, CreateResponse,
    FetchResponse, ListResponse, LoginResponse, LogoutResponse, LsubResponse, NoopResponse,
    SelectResponse, ServerErrorResponse, StatusResponse,
};
use imap_derive::ImapResponses;
use util::EncodeTo;

#[derive(Debug, ImapResponses)]
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

pub trait ServerResponseTrait<T>: EncodeTo {
    fn respond_to(cmd: T) -> Self;
}
