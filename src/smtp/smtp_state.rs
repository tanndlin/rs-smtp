use crate::smtp::{
    mail::Mail,
    message::{ExtendedHello, Hello, Message, Response},
};

#[derive(Default)]
pub struct SMTPState {
    domain: Option<String>,
    from: Option<String>,
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
        }
    }

    fn handle_extended_hello(&mut self, ehlo: ExtendedHello) -> Response {
        self.domain = Some(ehlo.domain);
        Response::Ok(())
    }

    fn handle_hello(&mut self, helo: Hello) -> Response {
        self.domain = Some(helo.domain);
        Response::Ok(())
    }

    fn handle_mail(&mut self, mail: Mail) -> Response {
        self.from = Some(mail.from);
        Response::Ok(())
    }
}
