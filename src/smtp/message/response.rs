use crate::{
    smtp::message::{Message, Ready},
    util::encode_to::EncodeTo,
};

pub enum Response {
    Ready(Ready),
}

impl EncodeTo for Response {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            Response::Ready(ready) => ready.encode_to(buf),
        };

        buf.extend(b"\r\n");
    }
}
