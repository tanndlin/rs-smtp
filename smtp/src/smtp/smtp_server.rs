use std::{
    fs,
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex, PoisonError},
    thread::{self, JoinHandle},
};

use amiquip::{Channel, Connection, Exchange, Publish};

use crate::{
    smtp::{
        message::{Ready, Request, Response},
        smtp_state::SMTPState,
    },
    util::{encode_to::EncodeTo, line_parser::LineParser},
};

pub struct SMTPServer {
    listen_thread: JoinHandle<()>,
}

impl SMTPServer {
    pub fn new(addr: SocketAddr, connection: Arc<Mutex<Connection>>) -> Result<Self, String> {
        let listener =
            TcpListener::bind(addr).map_err(|e| format!("Error creating tcp listener {e}"))?;

        println!("Listening on {addr}");
        let listen_thread = thread::spawn(move || listen(listener, connection));

        let mail_dir = Path::new("mail");
        if !mail_dir.exists() {
            fs::create_dir(mail_dir).unwrap();
        }

        Ok(Self { listen_thread })
    }

    pub fn join(self) {
        self.listen_thread.join().unwrap();
    }
}

fn listen(listener: TcpListener, connection: Arc<Mutex<Connection>>) {
    loop {
        let (stream, addr) = listener.accept().unwrap();
        let connection = connection.clone();
        thread::spawn(move || handle_request(stream, addr, connection));
    }
}

fn handle_request(mut stream: TcpStream, addr: SocketAddr, connection: Arc<Mutex<Connection>>) {
    let channel = connection
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open_channel(None)
        .unwrap();

    let mut state = SMTPState::new(move |mail| handle_mail_received(&mail, &channel));
    let mut line_parser = LineParser::new(stream.try_clone().unwrap());
    #[cfg(debug_assertions)]
    println!("Connected to client: {addr}");

    let ready = Response::Ready(Ready::new());
    ready.write_to(&mut stream).unwrap();

    while let Ok(message) = line_parser.next_line() {
        if !state.receiving_data {
            let command = Request::try_from(message).unwrap();
            let response = state.handle_message(command);

            if matches!(response, Response::Closing(())) {
                response.write_to(&mut stream).unwrap();
                break;
            }

            response.write_to(&mut stream).unwrap();
        } else if let Some(res) = state.handle_data_content(message) {
            res.write_to(&mut stream).unwrap();
        }
    }

    #[cfg(debug_assertions)]
    dbg!("Connection closed with peer: {addr}");
}

fn handle_mail_received(mail: &str, channel: &Channel) {
    Exchange::direct(channel)
        .publish(Publish::new(mail.as_bytes(), "mail"))
        .unwrap();
}
