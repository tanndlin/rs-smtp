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
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use imap::imap_server::IMAPServer;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, Connection, PgConnection};

/// Connection options for the Postgres instance the tests create their
/// throwaway databases on. Override with `TEST_DATABASE_URL` (falls back to
/// `DATABASE_URL`); the default matches the `db` service in `docker-compose.yml`.
fn base_connect_options() -> PgConnectOptions {
    let url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://user:password@localhost:5432/postgres".to_string());
    url.parse()
        .expect("TEST_DATABASE_URL / DATABASE_URL is not a valid Postgres URL")
}

fn unique_db_name() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "imap_test_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// A running server bound to a freshly migrated, throwaway database.
///
/// The database is dropped when this value goes out of scope, so bind it for
/// the whole test (`let server = start_server().await;`) rather than letting it
/// drop straight away. Derefs to the [`SocketAddr`] to connect to.
pub struct TestServer {
    pub addr: SocketAddr,
    admin: PgConnectOptions,
    db_name: String,
}

impl Deref for TestServer {
    type Target = SocketAddr;

    fn deref(&self) -> &SocketAddr {
        &self.addr
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let admin = self.admin.clone();
        let db_name = std::mem::take(&mut self.db_name);
        if db_name.is_empty() {
            return;
        }

        // `Drop` can't await, so run the teardown on a throwaway runtime. FORCE
        // kicks the server's still-open pool connections off the database.
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build teardown runtime");
            rt.block_on(async move {
                let Ok(mut conn) = PgConnection::connect_with(&admin).await else {
                    return;
                };
                // `db_name` is process-generated (see `unique_db_name`), not user input.
                let _ = sqlx::query(AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"
                )))
                .execute(&mut conn)
                .await;
            });
        })
        .join();
    }
}

/// Create a fresh database, run the migrations into it, bind the server to an
/// OS-assigned loopback port, and run its accept loop on a background task.
pub async fn start_server() -> TestServer {
    let admin = base_connect_options().database("postgres");
    let db_name = unique_db_name();

    let mut conn = PgConnection::connect_with(&admin)
        .await
        .expect("failed to connect to Postgres for test-db setup");
    // `db_name` is process-generated (see `unique_db_name`), not user input.
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE \"{db_name}\"")))
        .execute(&mut conn)
        .await
        .expect("failed to create test database");
    conn.close().await.ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(base_connect_options().database(&db_name))
        .await
        .expect("failed to connect to test database");
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("failed to migrate test database");

    let server = IMAPServer::new("127.0.0.1:0".parse().unwrap(), Arc::new(pool)).await;
    let addr = server.local_addr().expect("no local addr");
    tokio::spawn(async move { server.start().await });

    TestServer {
        addr,
        admin,
        db_name,
    }
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
