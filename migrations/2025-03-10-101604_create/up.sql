-- Your SQL goes here
ALTER TABLE emails ADD COLUMN is_sent BOOLEAN NOT NULL DEFAULT FALSE;
