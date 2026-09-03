use crate::smtp::message::Ready;
use util::EncodeTo;

pub enum Response {
    Ready(Ready),
    Ok,
    Closing,
    StartMailInput,
    Unrecognized,
}

impl EncodeTo for Response {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            Response::Ready(ready) => ready.encode_to(buf),
            Response::Ok => buf.extend_from_slice(b"250 OK"),
            Response::Closing => buf.extend_from_slice(b"221 rs-smtp v0.1 closing channel"),
            Response::StartMailInput => buf.extend_from_slice(b"354 start mail input"),
            Response::Unrecognized => buf.extend_from_slice(b"500 5.5.1 Command unrecognized"),
        }

        buf.extend_from_slice(b"\r\n");
    }
}
