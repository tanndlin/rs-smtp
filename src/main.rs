// https://datatracker.ietf.org/doc/html/rfc5321

use crate::smtp::SMTPServer;

mod smtp;

fn main() {
    let server = SMTPServer::new("0.0.0.0:2525".parse().expect("Invalid address"))
        .expect("Failed to start SMTP server");

    server.join();
}
