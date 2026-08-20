use crate::smtp::message::{ExtendedHello, Hello, Message, Response};

pub struct SMTPState {
    domain: Option<String>,
}

impl SMTPState {
    pub fn new() -> Self {
        Self { domain: None }
    }

    pub fn handle_message(&mut self, message: Message) -> Response {
        dbg!(&message);

        match message {
            Message::HELO(helo) => self.handle_hello(helo),
            Message::EHLO(ehlo) => self.handle_extended_hello(ehlo),
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
}
