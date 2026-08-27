use crate::smtp::message::Ready;
use util::EncodeTo;

pub enum Response {
    Ready(Ready),
    Ok(()),
    Closing(()),
    StartMailInput(()),
}

impl EncodeTo for Response {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            Response::Ready(ready) => ready.encode_to(buf),
            Response::Ok(()) => buf.extend(b"250 OK"),
            Response::Closing(()) => buf.extend(b"221 rs-smtp v0.1 closing channel"),
            Response::StartMailInput(()) => buf.extend(b"354 start mail input"),
        }

        buf.extend(b"\r\n");
    }
}
