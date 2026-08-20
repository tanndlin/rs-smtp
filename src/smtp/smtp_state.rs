use crate::smtp::{
    mail::MailMessage,
    message::{ExtendedHelloMessage, HelloMessage, Message, RecipientMessage, Response},
};

#[derive(Default)]
pub struct SMTPState {
    domain: Option<String>,
    from: Option<String>,
    recipient: Option<String>,
    pub receiving_data: bool, // Whether we are receiving data from client
    data: Vec<String>,
}

impl SMTPState {
    pub fn new() -> Self {
        Self::default()
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
            Some(Response::Ok(()))
        } else {
            None
        }
    }

    pub fn handle_message(&mut self, message: Message) -> Response {
        dbg!(&message);

        match message {
            Message::Hello(helo) => self.handle_hello(helo),
            Message::EHello(ehlo) => self.handle_extended_hello(ehlo),
            Message::Mail(mail) => self.handle_mail(mail),
            Message::Recipient(recipient) => self.handle_recipient(recipient),
            Message::Data(_) => self.handle_data_command(),
            Message::Quit(_) => self.handle_quit(),
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
