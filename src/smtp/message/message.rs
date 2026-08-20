use crate::{
    smtp::message::{extended_hello::ExtendedHello, hello::Hello, ready::Ready},
    util::encode_to::EncodeTo,
};

#[derive(Debug)]
pub enum Message {
    HELO(Hello),
    EHLO(ExtendedHello),
}

impl TryFrom<String> for Message {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (command, rest) = value.split_once(" ").expect("No spaces in message"); // TODO: THere is probably a valid message spec with only command
        Ok(match command {
            "EHLO" => Message::HELO(Hello::from(rest)),
            "HELO" => Message::EHLO(ExtendedHello::from(rest)),
            _ => return Err(format!("Unknown command: {command}")),
        })
    }
}
