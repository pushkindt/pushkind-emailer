-- Your SQL goes here
CREATE TABLE IF NOT EXISTS recipient_fields (
    recipient_id INTEGER NOT NULL REFERENCES recipients(id),
    field VARCHAR(32) NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (recipient_id, field)
);
