-- Your SQL goes here
ALTER TABLE email_recipients ADD COLUMN is_sent BOOLEAN NOT NULL DEFAULT FALSE;