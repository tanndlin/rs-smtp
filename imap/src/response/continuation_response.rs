use util::EncodeTo;

use crate::{response::ServerResponse, server_response_from_impl};

#[derive(Debug)]
pub struct ContinuationResponse {
    pub tag: String,
}

impl EncodeTo for ContinuationResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        buf.extend(b"+ pls\r\n");
    }
}

server_response_from_impl!(ContinuationResponse, Continue);
