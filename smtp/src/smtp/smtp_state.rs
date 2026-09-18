use crate::smtp::{
    email::Email,
    message::{
        ExtendedHelloMessage, HelloMessage, MailMessage, RecipientMessage, Request, Response,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Init,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
}

pub struct SMTPState {
    phase: Phase,
    domain: Option<String>,
    from: Option<String>,
    recipient: Vec<String>,
    data: Vec<String>,
    received_callback: Box<dyn FnMut(Email)>,
}

impl SMTPState {
    pub fn new(received_callback: impl FnMut(Email) + 'static) -> Self {
        Self {
            phase: Phase::Init,
            domain: None,
            from: None,
            recipient: vec![],
            data: Vec::new(),
            received_callback: Box::new(received_callback),
        }
    }

    pub fn handle_line(&mut self, line: &str) -> Option<Response> {
        if self.phase == Phase::Data {
            return self.handle_data_content(line);
        }

        Some(match Request::try_from(line) {
            Ok(command) => self.handle_message(command),
            Err(_) => Response::Unrecognized,
        })
    }

    fn handle_data_content(&mut self, data: &str) -> Option<Response> {
        if data == ".\r\n" {
            let email = Email::from(&*self);
            (self.received_callback)(email);
            self.reset_transaction();
            return Some(Response::Ok);
        }

        // dot-stuffing (RFC 5321 4.5.2).
        let line = data.strip_prefix('.').map_or(data, |rest| rest);
        self.data.push(line.to_string());
        None
    }

    fn handle_message(&mut self, message: Request) -> Response {
        match message {
            Request::Hello(helo) => self.handle_hello(helo),
            Request::EHello(ehlo) => self.handle_extended_hello(ehlo),
            Request::Mail(mail) => self.handle_mail(mail),
            Request::Recipient(recipient) => self.handle_recipient(recipient),
            Request::Data => self.handle_data_command(),
            Request::Reset => self.handle_reset(),
            Request::Noop => Response::Ok,
            Request::Quit => Response::Closing,
        }
    }

    fn reset_transaction(&mut self) {
        self.from = None;
        self.recipient.clear();
        self.data.clear();
        if self.phase != Phase::Init {
            self.phase = Phase::Greeted;
        }
    }

    fn handle_reset(&mut self) -> Response {
        self.reset_transaction();
        Response::Ok
    }

    fn greet(&mut self, domain: String) -> Response {
        self.domain = Some(domain);
        self.reset_transaction();
        self.phase = Phase::Greeted;
        Response::Ok
    }

    fn handle_extended_hello(&mut self, ehlo: ExtendedHelloMessage) -> Response {
        self.greet(ehlo.domain)
    }

    fn handle_hello(&mut self, helo: HelloMessage) -> Response {
        self.greet(helo.domain)
    }

    fn handle_mail(&mut self, mail: MailMessage) -> Response {
        if self.phase != Phase::Greeted {
            return Response::BadSequence;
        }

        self.from = Some(mail.from);
        self.phase = Phase::MailFrom;
        Response::Ok
    }

    fn handle_recipient(&mut self, mail: RecipientMessage) -> Response {
        if !matches!(self.phase, Phase::MailFrom | Phase::RcptTo) {
            return Response::BadSequence;
        }

        self.recipient.push(mail.to);
        self.phase = Phase::RcptTo;
        Response::Ok
    }

    fn handle_data_command(&mut self) -> Response {
        if self.phase != Phase::RcptTo {
            return Response::BadSequence;
        }

        self.phase = Phase::Data;
        Response::StartMailInput
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
