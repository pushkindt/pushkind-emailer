-- Your SQL goes here
DELETE FROM email_recipients;
DELETE FROM emails;
DROP TABLE IF EXISTS users;
ALTER TABLE hubs DROP COLUMN name;
ALTER TABLE emails DROP COLUMN user_id;
ALTER TABLE emails ADD COLUMN hub_id INTEGER NOT NULL REFERENCES hubs(id);
