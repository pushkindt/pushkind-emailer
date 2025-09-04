-- Your SQL goes here
ALTER TABLE email_recipients DROP COLUMN name;
ALTER TABLE email_recipients ADD COLUMN name VARCHAR(255) NOT NULL DEFAULT "";
ALTER TABLE email_recipients ADD COLUMN fields TEXT NOT NULL DEFAULT "";
