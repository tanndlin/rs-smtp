use crate::{smtp::message::Ready, util::encode_to::EncodeTo};

pub enum Response {
    Ready(Ready),
    Ok(()),
}

impl EncodeTo for Response {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            Response::Ready(ready) => ready.encode_to(buf),
            Response::Ok(_) => buf.extend(b"250 OK\r\n"),
        };

        buf.extend(b"\r\n");
    }
}
