use std::{
    collections::BTreeMap,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex, PoisonError},
    thread::{self},
};

use amiquip::{AmqpProperties, AmqpValue, Channel, Connection, Exchange, Publish};

use crate::{
    smtp::{
        email::Email,
        message::{Ready, Response},
        smtp_state::SMTPState,
    },
    util::line_parser::LineParser,
};
use util::EncodeTo;

pub struct SMTPServer {
    addr: SocketAddr,
    connection: Arc<Mutex<Connection>>,
}

impl SMTPServer {
    pub fn new(addr: SocketAddr, connection: Arc<Mutex<Connection>>) -> Self {
        Self { addr, connection }
    }

    pub fn listen(self) -> Result<!, String> {
        let listener =
            TcpListener::bind(self.addr).map_err(|e| format!("Error creating tcp listener {e}"))?;
        println!("Listening on {}", self.addr);

        loop {
            let (stream, addr) = match listener.accept() {
                Ok(conn) => conn,
                Err(e) => {
                    println!("Failed to accept connection: {e}");
                    continue;
                }
            };
            let connection = self.connection.clone();
            thread::spawn(move || handle_request(stream, addr, &connection));
        }
    }
}

fn handle_request(mut stream: TcpStream, addr: SocketAddr, connection: &Arc<Mutex<Connection>>) {
    let channel = connection
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open_channel(None)
        .unwrap();

    let mut state = SMTPState::new(move |email| handle_mail_received(email, &channel));
    let mut line_parser = LineParser::new(stream.try_clone().unwrap());
    println!("[{addr}] connected");

    let ready = Response::Ready(Ready::new());
    ready.write_to(&mut stream).unwrap();

    while let Ok(line) = line_parser.next_line() {
        let Some(response) = state.handle_line(&line) else {
            continue;
        };

        match response {
            Response::Unrecognized => println!("[{addr}] Unknown command: {}", line.trim_end()),
            Response::BadSequence => println!("[{addr}] Out of sequence: {}", line.trim_end()),
            _ => {}
        }

        let closing = matches!(response, Response::Closing);
        response.write_to(&mut stream).unwrap();
        if closing {
            break;
        }
    }

    println!("[{addr}] connection closed");
}

fn handle_mail_received(email: Email, channel: &Channel) {
    println!(
        "accepted mail from <{}> for [{}] ({} bytes)",
        email.from,
        email.to.join(", "),
        email.data.len()
    );

    let mut headers = BTreeMap::new();
    headers.insert("from".to_string(), AmqpValue::LongString(email.from));
    headers.insert(
        "recipients".to_string(),
        AmqpValue::LongString(email.to.join(";")),
    );

    // Get current timestamp in RFC 3339 format
    let received_at = chrono::Utc::now().to_rfc3339();
    headers.insert(
        "received_at".to_string(),
        AmqpValue::LongString(received_at),
    );

    let properties = AmqpProperties::default().with_headers(headers);
    let payload = email.data.as_bytes();
    Exchange::direct(channel)
        .publish(Publish::with_properties(payload, "mail", properties))
        .unwrap();
}
