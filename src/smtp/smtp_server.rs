use std::{
    io::Write,
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

use crate::{
    smtp::{
        message::{Message, Ready, Response},
        smtp_state::SMTPState,
    },
    util::{encode_to::EncodeTo, line_parser::LineParser},
};

pub struct SMTPServer {
    listen_thread: JoinHandle<()>,
}

impl SMTPServer {
    pub fn new(addr: SocketAddr) -> Result<Self, String> {
        let listener =
            TcpListener::bind(addr).map_err(|e| format!("Error creating tcp listener {e}"))?;

        println!("Listening on {addr}");
        let listen_thread = thread::spawn(|| listen(listener));

        Ok(Self { listen_thread })
    }

    pub fn join(self) {
        self.listen_thread.join().unwrap();
    }
}

fn listen(listener: TcpListener) {
    loop {
        let (stream, addr) = listener.accept().unwrap();
        thread::spawn(move || handle_request(stream, addr));
    }
}

fn handle_request(mut stream: TcpStream, addr: SocketAddr) {
    let mut state = SMTPState::new();
    let mut line_parser = LineParser::new(stream.try_clone().unwrap());
    println!("Connected to client: {addr}");

    let ready = Response::Ready(Ready::new());
    ready.write_to(&mut stream).unwrap();

    loop {
        let message = line_parser.next_line().unwrap();
        dbg!(&message);
        if !state.receiving_data {
            let command = Message::try_from(message).unwrap();
            let response = state.handle_message(command);

            if matches!(response, Response::Closing(_)) {
                response.write_to(&mut stream).unwrap();
                break;
            }

            response.write_to(&mut stream).unwrap();
        } else {
            if let Some(res) = state.handle_data_content(message) {
                res.write_to(&mut stream).unwrap();
            }
        }
    }

    println!("Connection closed with peer: {addr}");
}
