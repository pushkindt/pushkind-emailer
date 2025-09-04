-- This file should undo anything in `up.sql`
ALTER TABLE email_recipients DROP COLUMN name;
ALTER TABLE email_recipients ADD COLUMN name VARCHAR(255);
ALTER TABLE email_recipients DROP COLUMN fields;
