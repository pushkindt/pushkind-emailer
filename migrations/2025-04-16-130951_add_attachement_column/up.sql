-- Your SQL goes here
ALTER TABLE EMAILS ADD COLUMN attachment BLOB;
ALTER TABLE EMAILS ADD COLUMN attachment_name TEXT;
ALTER TABLE EMAILS ADD COLUMN attachment_mime TEXT;
