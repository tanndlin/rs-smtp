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

    let server = SMTPServer::new(bind, connection.clone()).expect("Failed to start SMTP server");

    server.join();
    Arc::try_unwrap(connection)
        .expect("connection still in use")
        .into_inner()
        .unwrap()
        .close()
        .unwrap();
}
