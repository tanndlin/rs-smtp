use crate::smtp::{
    mail::MailMessage,
    message::{ExtendedHelloMessage, HelloMessage, Message, RecipientMessage, Response},
};

#[derive(Default)]
pub struct SMTPState {
    domain: Option<String>,
    from: Option<String>,
    recipient: Option<String>,
}

impl SMTPState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_message(&mut self, message: Message) -> Response {
        dbg!(&message);

        match message {
            Message::Hello(helo) => self.handle_hello(helo),
            Message::EHello(ehlo) => self.handle_extended_hello(ehlo),
            Message::Mail(mail) => self.handle_mail(mail),
            Message::Recipient(recipient) => self.handle_recipient(recipient),
        }
    }

    fn handle_extended_hello(&mut self, ehlo: ExtendedHelloMessage) -> Response {
        self.domain = Some(ehlo.domain);
        Response::Ok(())
    }

    fn handle_hello(&mut self, helo: HelloMessage) -> Response {
        self.domain = Some(helo.domain);
        Response::Ok(())
    }

    fn handle_mail(&mut self, mail: MailMessage) -> Response {
        self.from = Some(mail.from);
        Response::Ok(())
    }

    fn handle_recipient(&mut self, mail: RecipientMessage) -> Response {
        self.recipient = Some(mail.to);
        Response::Ok(())
    }
}
