-- This file should undo anything in `up.sql`
ALTER TABLE emails DROP COLUMN num_sent;
ALTER TABLE emails DROP COLUMN num_opened;
ALTER TABLE emails DROP COLUMN num_replied;
