-- Drop FTS triggers and table
DROP TRIGGER IF EXISTS recipients_ai;
DROP TRIGGER IF EXISTS recipients_au;
DROP TRIGGER IF EXISTS recipients_ad;
DROP TABLE IF EXISTS recipient_fts;
DROP TABLE IF EXISTS recipient_fts_data;
DROP TABLE IF EXISTS recipient_fts_idx;
DROP TABLE IF EXISTS recipient_fts_docsize;
DROP TABLE IF EXISTS recipient_fts_config;

-- Remove the denormalized column
ALTER TABLE recipients DROP COLUMN fields;