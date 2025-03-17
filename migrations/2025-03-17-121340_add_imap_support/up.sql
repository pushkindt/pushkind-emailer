-- Your SQL goes here
ALTER TABLE hubs RENAME COLUMN server to smtp_server;
ALTER TABLE hubs RENAME COLUMN port to smtp_port;

ALTER TABLE hubs ADD COLUMN imap_server TEXT;
ALTER TABLE hubs ADD COLUMN imap_port INTEGER;

ALTER TABLE email_recipients ADD COLUMN replied BOOLEAN NOT NULL DEFAULT FALSE;
