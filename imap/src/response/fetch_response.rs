use std::{collections::HashMap, fmt};

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
        buf.extend_from_slice(format!("{tag} OK FETCH completed\r\n").as_bytes());
    }
}

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
        buf.extend_from_slice(
            format!(
                "* {message_id} FETCH ({})\r\n",
                self.metadata
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .as_bytes(),
        );
    }
}

/// A metadata value can hold a whole message body, so `Debug` prints only the
/// first `DEBUG_VALUE_LIMIT` characters of each one.
const DEBUG_VALUE_LIMIT: usize = 80;

struct TruncatedValue<'a>(&'a str);

impl fmt::Debug for TruncatedValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        match value.char_indices().nth(DEBUG_VALUE_LIMIT) {
            Some((end, _)) => write!(f, "{:?}... ({} bytes total)", &value[..end], value.len()),
            None => write!(f, "{value:?}"),
        }
    }
}

struct TruncatedMetadata<'a>(&'a HashMap<String, String>);

impl fmt::Debug for TruncatedMetadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (k, TruncatedValue(v))))
            .finish()
    }
}

impl fmt::Debug for FetchMessageResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FetchMessageResponse")
            .field("message_id", &self.message_id)
            .field("metadata", &TruncatedMetadata(&self.metadata))
            .finish()
    }
}

server_response_from_impl!(FetchResponse, Fetch);
