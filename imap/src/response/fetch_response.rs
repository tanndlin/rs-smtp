use std::collections::HashMap;

use util::EncodeTo;

use crate::{response::ServerResponse, server_response_from_impl};

#[derive(Debug)]
pub struct FetchResponse {
    request_tag: String,
    responses: Vec<FetchMessageResponse>,
}

impl FetchResponse {
    pub fn new(request_tag: String, responses: Vec<FetchMessageResponse>) -> Self {
        Self {
            request_tag,
            responses,
        }
    }
}

impl EncodeTo for FetchResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        for res in self.responses {
            res.encode_to(buf);
        }

        let tag = self.request_tag;
        buf.extend(format!("{tag} OK FETCH completed\r\n").bytes());
    }
}

#[derive(Debug)]
pub struct FetchMessageResponse {
    message_id: u64,
    metadata: HashMap<String, String>,
}

impl FetchMessageResponse {
    pub fn new(message_id: u64, metadata: HashMap<String, String>) -> Self {
        Self {
            message_id,
            metadata,
        }
    }
}

impl EncodeTo for FetchMessageResponse {
    fn encode_to(self, buf: &mut Vec<u8>) {
        let message_id = self.message_id;
        buf.extend(
            format!(
                "* {message_id} FETCH ({})\r\n",
                self.metadata
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .bytes(),
        );
    }
}

server_response_from_impl!(FetchResponse, Fetch);
