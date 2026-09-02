use std::sync::Arc;

use sqlx::{Pool, Postgres};

use crate::command::Fetchable;

// Returns the fetchable result as a string to encode to response
pub async fn get_fetchable(
    db_pool: Arc<Pool<Postgres>>,
    message_id: u64,
    fetchable: &Fetchable,
) -> String {
    match fetchable {
        Fetchable::All => todo!(),
        Fetchable::Fast => todo!(),
        Fetchable::Full => todo!(),
        Fetchable::Binary(binary_fetchable) => todo!(),
        Fetchable::Body(body_fetchable) => todo!(),
        Fetchable::BodyStructure => todo!(),
        Fetchable::Envelope => todo!(),
        Fetchable::Flags => todo!(),
        Fetchable::Internaldate => todo!(),
        Fetchable::RFC822Size => get_rfc822_size(db_pool, message_id).await,
        Fetchable::UID => get_uid(db_pool, message_id).await,
    }
}

// TODO: Fetch the mail once and pass through
async fn get_uid(db_pool: Arc<Pool<Postgres>>, message_id: u64) -> String {
    let uid = sqlx::query!("SELECT uid from mail WHERE id = $1", message_id as i32)
        .fetch_one(&*db_pool)
        .await
        .unwrap()
        .uid; // TODO: Check for 404

    uid.to_string()
}

async fn get_rfc822_size(db_pool: Arc<Pool<Postgres>>, message_id: u64) -> String {
    let raw = sqlx::query!("SELECT raw_eml from mail WHERE id = $1", message_id as i32)
        .fetch_one(&*db_pool)
        .await
        .unwrap()
        .raw_eml
        .unwrap();

    raw.len().to_string()
}
