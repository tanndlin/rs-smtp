use futures_lite::stream::StreamExt;
use lapin::{Connection, ConnectionProperties, options::*, types::FieldTable};
use sqlx::postgres::PgPoolOptions;

use util::Email;

#[tokio::main]
async fn main() {
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

    let rmq_connection = Connection::connect("amqp://rabbitmq", ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");
    let channel = rmq_connection
        .create_channel()
        .await
        .expect("Failed to open channel");

    channel
        .queue_declare(
            "mail".into(),
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .expect("Failed to declare a queue");

    let mut consumer = channel
        .basic_consume(
            "mail".into(),
            "consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("Failed to start consuming messages");

    println!("Waiting for messages. Press Ctrl-C to exit.");

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("Failed to receive message");

        let headers = delivery.properties.headers().as_ref().unwrap();
        let from = headers
            .inner()
            .get("from")
            .unwrap()
            .as_long_string()
            .unwrap()
            .to_string();
        let recipients: Vec<String> = headers
            .inner()
            .get("recipients")
            .unwrap()
            .as_long_string()
            .unwrap()
            .to_string()
            .split(";")
            .map(|s| s.to_string())
            .collect();

        let body = String::from_utf8_lossy(&delivery.data).into_owned();
        let email = Email::new(from, recipients, body);

        println!("Received message");
        println!("From: {}", email.from);
        println!("To: {}", email.recipients_to.join(", "));
        println!("----------------- Body -----------------");
        println!("{}", email.body_text.as_deref().unwrap_or_default());

        let mut tx = db_pool.begin().await.unwrap();
        let uid = sqlx::query_scalar!(
            "UPDATE mailboxes SET uid_next = uid_next + 1 WHERE name = $1 RETURNING (uid_next - 1) AS \"uid!\"",
            "INBOX",
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();

        match email.insert(&mut *tx, "INBOX", uid).await {
            Ok(()) => {
                if let Err(err) = tx.commit().await {
                    eprintln!("Failed to commit mail transaction: {err}");
                }
            }
            Err(err) => {
                eprintln!("Failed to insert mail into database: {err}");
                let _ = tx.rollback().await;
            }
        }

        delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to acknowledge message");
    }
}
