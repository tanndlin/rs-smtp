use crate::smtp::{
    mail::MailMessage,
    message::{ExtendedHelloMessage, HelloMessage, RecipientMessage, Request, Response},
};

pub struct SMTPState {
    domain: Option<String>,
    from: Option<String>,
    recipient: Option<String>,
    pub receiving_data: bool, // Whether we are receiving data from client
    data: Vec<String>,
    received_callback: fn(String),
}

impl SMTPState {
    pub fn new(received_callback: fn(String)) -> Self {
        Self {
            domain: None,
            from: None,
            recipient: None,
            receiving_data: false,
            data: Vec::new(),
            received_callback,
        }
    }

    pub fn handle_data_content(&mut self, data: String) -> Option<Response> {
        // TODO: I hate this
        self.data.push(data);

        if self.data.last().unwrap() == ".\r\n"
            && let Some(second_to_last) = self.data.iter().nth_back(1)
            && second_to_last == "\r\n"
        {
            self.data.pop();
            self.data.pop();

            self.receiving_data = false;
            (self.received_callback)(self.data.join(""));
            Some(Response::Ok(()))
        } else {
            None
        }
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
        self.recipient = Some(mail.to);
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
