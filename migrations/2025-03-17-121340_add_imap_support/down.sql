-- This file should undo anything in `up.sql`
ALTER TABLE hubs RENAME COLUMN smtp_server to server;
ALTER TABLE hubs RENAME COLUMN smtp_port to port;

ALTER TABLE hubs DROP COLUMN imap_server;
ALTER TABLE hubs DROP COLUMN imap_port;

ALTER TABLE email_recipients DROP COLUMN replied;