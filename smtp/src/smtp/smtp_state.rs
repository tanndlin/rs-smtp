use std::str::FromStr;

use crate::smtp::{
    email::Email,
    message::{Request, Response},
};

/// Connected, no HELO/EHLO yet.
pub struct Init;

/// Client has identified itself; no transaction in progress.
pub struct Greeted {
    domain: String,
}

/// MAIL FROM accepted.
pub struct MailFrom {
    domain: String,
    from: String,
}

/// At least one RCPT TO accepted.
pub struct RcptTo {
    domain: String,
    from: String,
    recipients: Vec<String>,
}

/// DATA accepted; collecting message content until `.`.
pub struct Data {
    domain: String,
    from: String,
    recipients: Vec<String>,
    lines: Vec<String>,
}

impl Greeted {
    fn new(domain: String) -> Greeted {
        Greeted { domain }
    }

    fn mail(self, from: String) -> MailFrom {
        MailFrom {
            domain: self.domain,
            from,
        }
    }
}

impl MailFrom {
    fn recipient(self, to: String) -> RcptTo {
        RcptTo {
            domain: self.domain,
            from: self.from,
            recipients: vec![to],
        }
    }

    fn reset(self) -> Greeted {
        Greeted {
            domain: self.domain,
        }
    }
}

impl RcptTo {
    fn recipient(mut self, to: String) -> RcptTo {
        self.recipients.push(to);
        self
    }

    fn data(self) -> Data {
        Data {
            domain: self.domain,
            from: self.from,
            recipients: self.recipients,
            lines: Vec::new(),
        }
    }

    fn reset(self) -> Greeted {
        Greeted {
            domain: self.domain,
        }
    }
}

impl Data {
    fn push_line(&mut self, line: &str) {
        // dot-stuffing (RFC 5321 4.5.2).
        let line = line.strip_prefix('.').unwrap_or(line);
        self.lines.push(line.to_string());
    }

    fn finish(self) -> (Greeted, Email) {
        let email = Email {
            from: self.from,
            to: self.recipients,
            data: self.lines.concat(),
        };
        let greeted = Greeted {
            domain: self.domain,
        };
        (greeted, email)
    }
}

enum Phase {
    Init(Init),
    Greeted(Greeted),
    MailFrom(MailFrom),
    RcptTo(RcptTo),
    Data(Data),
}

impl Phase {
    fn handle_request(self, request: Request) -> (Phase, Response) {
        match (self, request) {
            // HELO/EHLO are valid in any state and abort any open transaction.
            (_, Request::Hello(helo)) => (Phase::Greeted(Greeted::new(helo.domain)), Response::Ok),
            (_, Request::EHello(ehlo)) => (Phase::Greeted(Greeted::new(ehlo.domain)), Response::Ok),

            (Phase::Greeted(s), Request::Mail(mail)) => {
                (Phase::MailFrom(s.mail(mail.from)), Response::Ok)
            }
            (Phase::MailFrom(s), Request::Recipient(rcpt)) => {
                (Phase::RcptTo(s.recipient(rcpt.to)), Response::Ok)
            }
            (Phase::RcptTo(s), Request::Recipient(rcpt)) => {
                (Phase::RcptTo(s.recipient(rcpt.to)), Response::Ok)
            }
            (Phase::RcptTo(s), Request::Data) => (Phase::Data(s.data()), Response::StartMailInput),

            (phase, Request::Reset) => (phase.reset(), Response::Ok),
            (phase, Request::Noop) => (phase, Response::Ok),
            (phase, Request::Quit) => (phase, Response::Closing),

            (phase, Request::Mail(_) | Request::Recipient(_) | Request::Data) => {
                (phase, Response::BadSequence)
            }
        }
    }

    fn reset(self) -> Phase {
        match self {
            Phase::Init(s) => Phase::Init(s),
            Phase::Greeted(s) => Phase::Greeted(s),
            Phase::MailFrom(s) => Phase::Greeted(s.reset()),
            Phase::RcptTo(s) => Phase::Greeted(s.reset()),
            Phase::Data(s) => Phase::Greeted(s.finish().0),
        }
    }
}

pub struct SMTPSession {
    phase: Phase,
    received_callback: Box<dyn FnMut(Email)>,
}

impl SMTPSession {
    pub fn new(received_callback: impl FnMut(Email) + 'static) -> Self {
        Self {
            phase: Phase::Init(Init),
            received_callback: Box::new(received_callback),
        }
    }

    pub fn handle_line(&mut self, line: &str) -> Option<Response> {
        // Take ownership of the current state so transitions can consume it.
        let phase = std::mem::replace(&mut self.phase, Phase::Init(Init));

        let (next, response) = match phase {
            Phase::Data(data) => self.handle_data_line(data, line),
            phase => match Request::from_str(line) {
                Ok(request) => {
                    let (next, response) = phase.handle_request(request);
                    (next, Some(response))
                }
                Err(_) => (phase, Some(Response::Unrecognized)),
            },
        };

        self.phase = next;
        response
    }

    fn handle_data_line(&mut self, mut data: Data, line: &str) -> (Phase, Option<Response>) {
        if line == ".\r\n" {
            let (greeted, email) = data.finish();
            (self.received_callback)(email);
            return (Phase::Greeted(greeted), Some(Response::Ok));
        }

        data.push_line(line);
        (Phase::Data(data), None)
    }
}
