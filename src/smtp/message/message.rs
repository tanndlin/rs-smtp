use crate::smtp::{
    mail::Mail,
    message::{extended_hello::ExtendedHello, hello::Hello},
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Message {
    Hello(Hello),
    EHello(ExtendedHello),
    Mail(Mail),
}

impl TryFrom<String> for Message {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (command, rest) = value.split_once(" ").expect("No spaces in message"); // TODO: THere is probably a valid message spec with only command
        Ok(match command {
            "EHLO" => Message::Hello(Hello::from(rest)),
            "HELO" => Message::EHello(ExtendedHello::from(rest)),
            "MAIL" => Message::Mail(Mail::from(rest)),
            _ => return Err(format!("Unknown command: {command}")),
        })
    }
}
