use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

use crate::imap_server::IMAPServer;

mod command;
mod imap_server;
mod imap_state;

#[tokio::main]
async fn main() {
    let ip = "0.0.0.0:143".parse().expect("Failed to parse IP");
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    println!("Connecting to database...");
    let db_pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("../migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    println!("Database connected and migrations applied.");

    let imap_server = IMAPServer::new(ip, Arc::new(db_pool));
    imap_server.start();
}
