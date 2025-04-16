-- This file should undo anything in `up.sql`
ALTER TABLE EMAILS DROP COLUMN attachment;
ALTER TABLE EMAILS DROP COLUMN attachment_name;
ALTER TABLE EMAILS DROP COLUMN attachment_mime;
