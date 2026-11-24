use std::str::FromStr;

use crate::smtp::message::{ExtendedHelloMessage, HelloMessage, MailMessage, RecipientMessage};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Request {
    Hello(HelloMessage),
    EHello(ExtendedHelloMessage),
    Mail(MailMessage),
    Recipient(RecipientMessage),
    Data,
    Reset,
    Noop,
    Quit,
}

impl FromStr for Request {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (command, rest) = value.split_once(' ').unwrap_or((value.trim(), ""));
        Ok(match command {
            "HELO" => Request::Hello(HelloMessage::from(rest)),
            "EHLO" => Request::EHello(ExtendedHelloMessage::from(rest)),
            "MAIL" => Request::Mail(MailMessage::from(rest)),
            "RCPT" => Request::Recipient(RecipientMessage::from(rest)),
            "DATA" => Request::Data,
            "RSET" => Request::Reset,
            "NOOP" => Request::Noop,
            "QUIT" => Request::Quit,
            _ => return Err(format!("Unknown command: {command}")),
        })
    }
}
