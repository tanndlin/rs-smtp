use crate::{
    smtp::message::{extended_hello::ExtendedHello, hello::Hello, ready::Ready},
    util::encode_to::EncodeTo,
};

pub enum Message {
    Ready(Ready),
    HELO(Hello),
    EHLO(ExtendedHello),
}

impl EncodeTo for Message {
    fn encode_to(self, buf: &mut Vec<u8>) {
        match self {
            Message::Ready(ready) => ready.encode_to(buf),
            Message::HELO(hello) => todo!(),
            Message::EHLO(extended_hello) => todo!(),
        };

        buf.extend(b"\r\n");
    }
}
