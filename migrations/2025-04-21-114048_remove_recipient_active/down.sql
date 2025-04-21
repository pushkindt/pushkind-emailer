-- This file should undo anything in `up.sql`
ALTER TABLE recipients ADD COLUMN active BOOLEAN NOT NULL DEFAULT TRUE;
