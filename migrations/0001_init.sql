CREATE TABLE IF NOT EXISTS "mailboxes" (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    uid_next INTEGER NOT NULL DEFAULT 1,
    uid_validity INTEGER NOT NULL DEFAULT extract(epoch FROM now())::int
);

INSERT INTO "mailboxes" (name) VALUES ('INBOX') ON CONFLICT (name) DO NOTHING;

CREATE TABLE IF NOT EXISTS "mail" (
    id SERIAL PRIMARY KEY,
    message_id VARCHAR(255) UNIQUE,
    in_reply_to VARCHAR(255),
    uid INTEGER NOT NULL,
    "from" VARCHAR(255) NOT NULL,
    sender VARCHAR(255),
    reply_to VARCHAR(255),
    recipients_to TEXT[] NOT NULL DEFAULT '{}',
    recipients_cc TEXT[] NOT NULL DEFAULT '{}',
    recipients_bcc TEXT[] NOT NULL DEFAULT '{}',
    subject VARCHAR(500),
    sent_date TIMESTAMPTZ,
    body_text TEXT,
    body_html TEXT,
    raw_eml TEXT NOT NULL,
    mailbox_id INTEGER REFERENCES "mailboxes"(id) ON DELETE CASCADE,
    CONSTRAINT mail_uid_unique UNIQUE (mailbox_id, uid)
);

CREATE TABLE IF NOT EXISTS "email_attachments" (
    id SERIAL PRIMARY KEY,
    email_id INT,
    file_name VARCHAR(255),
    content_type VARCHAR(100),
    file_size INT,
    file_data BYTEA,
    FOREIGN KEY (email_id) REFERENCES "mail"(id) ON DELETE CASCADE
);
