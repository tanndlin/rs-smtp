CREATE TABLE IF NOT EXISTS "mail" (
    id SERIAL PRIMARY KEY,
    message_id VARCHAR(255) UNIQUE,
    sender VARCHAR(255),
    recipient_to TEXT,
    recipient_cc TEXT,
    subject VARCHAR(500),
    sent_date TIMESTAMPTZ,
    body_text TEXT,
    body_html TEXT,
    raw_eml TEXT
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