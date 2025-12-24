-- This file should undo anything in `up.sql`
ALTER TABLE email_recipients ADD COLUMN replied BOOLEAN NOT NULL DEFAULT FALSE;
