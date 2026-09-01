use util::EncodeTo;

use crate::{response::ServerResponse, server_response_from_impl};

#[derive(Debug)]
pub struct AppendOkResponse {
    request_tag: String,
    uid_validity: u32,
    uid: u32,
}

impl AppendOkResponse {
    pub fn new(request_tag: String, uid_validity: u32, uid: u32) -> Self {
        Self {
            request_tag,
            uid_validity,
            uid,
        }
    }
}

server_response_from_impl!(AppendOkResponse, Append);

impl EncodeTo for AppendOkResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let AppendOkResponse {
            request_tag,
            uid_validity,
            uid,
        } = self;

        buf.extend(
            format!("{request_tag} OK [APPENDUID {uid_validity} {uid}] APPEND completed\r\n").bytes(),
        );
    }
}
