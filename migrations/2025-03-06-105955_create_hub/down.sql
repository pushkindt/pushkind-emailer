-- This file should undo anything in `up.sql`
ALTER TABLE users DROP COLUMN hub_id;
DROP TABLE IF EXISTS hubs;
