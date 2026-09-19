use sqlx::prelude::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Mailbox {
    pub id: i32,
    pub name: String,
    pub uid_next: i32, // UID the next message delivered into this mailbox will get
    pub uid_validity: i32,
}

impl Mailbox {
    /// Look up the mailbox called `name`, if there is one.
    pub async fn fetch<'e, E>(executor: E, name: &str) -> sqlx::Result<Option<Self>>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as!(
            Mailbox,
            "SELECT id, name, uid_next, uid_validity FROM mailboxes WHERE name = $1",
            name,
        )
        .fetch_optional(executor)
        .await
    }

    /// Create an empty mailbox called `name`.
    pub async fn create<'e, E>(executor: E, name: &str) -> sqlx::Result<Self>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_as!(
            Mailbox,
            "INSERT INTO mailboxes (name, uid_validity, uid_next) VALUES ($1, 1, 1)
             RETURNING id, name, uid_next, uid_validity",
            name,
        )
        .fetch_one(executor)
        .await
    }

    /// Reserve the next UID in the mailbox called `name`.
    /// Returns `(uid, uid_validity)`.
    pub async fn allocate_uid<'e, E>(executor: E, name: &str) -> sqlx::Result<(i32, i32)>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query!(
            r#"UPDATE mailboxes SET uid_next = uid_next + 1 WHERE name = $1
               RETURNING (uid_next - 1) AS "uid!", uid_validity"#,
            name,
        )
        .fetch_one(executor)
        .await
        .map(|row| (row.uid, row.uid_validity))
    }
}
