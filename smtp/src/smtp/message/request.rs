use crate::smtp::message::{ExtendedHelloMessage, HelloMessage, MailMessage, RecipientMessage};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Request {
    Hello(HelloMessage),
    EHello(ExtendedHelloMessage),
    Mail(MailMessage),
    Recipient(RecipientMessage),
    Data,
    Quit,
}

impl TryFrom<String> for Request {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (command, rest) = value.split_once(' ').unwrap_or((value.as_str().trim(), ""));
        Ok(match command {
            "HELO" => Request::Hello(HelloMessage::from(rest)),
            "EHLO" => Request::EHello(ExtendedHelloMessage::from(rest)),
            "MAIL" => Request::Mail(MailMessage::from(rest)),
            "RCPT" => Request::Recipient(RecipientMessage::from(rest)),
            "DATA" => Request::Data,
            "QUIT" => Request::Quit,
            _ => return Err(format!("Unknown command: {command}")),
        })
    }
}
