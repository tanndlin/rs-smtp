use crate::smtp::{
    mail::MailMessage,
    message::{
        extended_hello::ExtendedHelloMessage, hello::HelloMessage, recipient::RecipientMessage,
    },
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum Message {
    Hello(HelloMessage),
    EHello(ExtendedHelloMessage),
    Mail(MailMessage),
    Recipient(RecipientMessage),
    Data(()),
    Quit(()),
}

impl TryFrom<String> for Message {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (command, rest) = value.split_once(' ').unwrap_or((value.as_str().trim(), ""));
        Ok(match command {
            "EHLO" => Message::Hello(HelloMessage::from(rest)),
            "HELO" => Message::EHello(ExtendedHelloMessage::from(rest)),
            "MAIL" => Message::Mail(MailMessage::from(rest)),
            "RCPT" => Message::Recipient(RecipientMessage::from(rest)),
            "DATA" => Message::Data(()),
            "QUIT" => Message::Quit(()),
            _ => return Err(format!("Unknown command: {command}")),
        })
    }
}
