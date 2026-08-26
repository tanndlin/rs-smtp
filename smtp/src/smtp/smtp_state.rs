use crate::smtp::{
    email::Email,
    message::{
        ExtendedHelloMessage, HelloMessage, MailMessage, RecipientMessage, Request, Response,
    },
};

pub struct SMTPState {
    domain: Option<String>,
    from: Option<String>,
    recipient: Vec<String>,
    pub receiving_data: bool, // Whether we are receiving data from client
    data: Vec<String>,
    received_callback: Box<dyn FnMut(Email)>,
}

impl SMTPState {
    pub fn new(received_callback: impl FnMut(Email) + 'static) -> Self {
        Self {
            domain: None,
            from: None,
            recipient: vec![],
            receiving_data: false,
            data: Vec::new(),
            received_callback: Box::new(received_callback),
        }
    }

    pub fn handle_data_content(&mut self, data: String) -> Option<Response> {
        if data == ".\r\n" {
            self.receiving_data = false;
            let email = Email::from(&*self);
            (self.received_callback)(email);
            self.data.clear();
            return Some(Response::Ok(()));
        }

        // dot-stuffing (RFC 5321 4.5.2).
        let line = data.strip_prefix('.').map_or(data.as_str(), |rest| rest);
        self.data.push(line.to_string());
        None
    }

    pub fn handle_message(&mut self, message: Request) -> Response {
        match message {
            Request::Hello(helo) => self.handle_hello(helo),
            Request::EHello(ehlo) => self.handle_extended_hello(ehlo),
            Request::Mail(mail) => self.handle_mail(mail),
            Request::Recipient(recipient) => self.handle_recipient(recipient),
            Request::Data(()) => self.handle_data_command(),
            Request::Quit(()) => self.handle_quit(),
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
        self.recipient.push(mail.to);
        Response::Ok(())
    }

    fn handle_data_command(&mut self) -> Response {
        self.receiving_data = true;
        Response::StartMailInput(())
    }

    fn handle_quit(&self) -> Response {
        Response::Closing(())
    }
}

impl From<&SMTPState> for Email {
    fn from(value: &SMTPState) -> Self {
        Email {
            from: value.from.clone().unwrap(),
            to: value.recipient.clone(),
            data: value.data.join(""),
        }
    }
}
