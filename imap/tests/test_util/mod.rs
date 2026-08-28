//! Shared helpers for the integration tests.
//!
//! Lives in a subdirectory (`tests/test_util/mod.rs`) so Cargo does not compile
//! it as its own test binary. Pull it in from a test with `mod test_util;`.
//!
//! `allow(dead_code)`: each integration-test binary that does `mod test_util;`
//! only uses some of these, and unused ones would otherwise warn per-binary.
#![allow(dead_code)]

use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use imap::imap_server::IMAPServer;
use sqlx::postgres::PgPoolOptions;

/// Bind the server to an OS-assigned loopback port, run its accept loop on a
/// background thread, and return the address to connect to.
///
/// The pool is built lazily against a bogus URL: nothing in the request path
/// touches the database yet, and `connect_lazy` opens no connection.
pub fn start_server() -> SocketAddr {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused")
        .expect("failed to build lazy pool");

    let server = IMAPServer::new("127.0.0.1:0".parse().unwrap(), Arc::new(pool));
    let addr = server.local_addr().expect("no local addr");
    thread::spawn(move || server.start());
    addr
}

/// Read whatever the server has sent, stopping on a short idle gap so a
/// non-responding server fails the test instead of hanging it.
pub fn read_available(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();

    let mut out = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(_) => break, // timeout: nothing more for now
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
