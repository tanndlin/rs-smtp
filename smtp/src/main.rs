// https://datatracker.ietf.org/doc/html/rfc5321

use std::sync::{Arc, Mutex};

use amiquip::{Connection, QueueDeclareOptions};

use crate::smtp::SMTPServer;

mod smtp;
mod util;

fn main() {
    let mut connection = Connection::insecure_open("amqp://rabbitmq").unwrap();

    let channel = connection.open_channel(None).unwrap();
    channel
        .queue_declare(
            "mail",
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
        )
        .unwrap();
    channel.close().unwrap();

    let connection = Arc::new(Mutex::new(connection));
    let bind = "0.0.0.0:2525".parse().expect("Invalid address");

    let Err(e) = SMTPServer::new(bind, connection).listen();
    panic!("Failed to listen for incoming connections: {e}");
}
