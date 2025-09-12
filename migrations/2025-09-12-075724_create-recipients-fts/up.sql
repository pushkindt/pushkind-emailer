-- Add a column to store concatenated optional fields for FTS
ALTER TABLE recipients ADD COLUMN fields TEXT;

-- Backfill current values from recipient_fields
UPDATE recipients
SET fields = (
  SELECT trim(COALESCE(group_concat(value, ' '), ''))
  FROM recipient_fields rf
  WHERE rf.recipient_id = recipients.id
);

-- Recreate the FTS5 table including the new column
CREATE VIRTUAL TABLE recipient_fts USING fts5(
    name,
    email,
    fields,
    content='recipients',
    content_rowid='id',
    tokenize = 'unicode61'
);

-- Populate FTS from content table
INSERT INTO recipient_fts(recipient_fts) VALUES('rebuild');

-- Recreate triggers on recipients to maintain FTS
CREATE TRIGGER recipients_ai AFTER INSERT ON recipients BEGIN
  INSERT INTO recipient_fts(rowid, name, email, fields)
  VALUES (new.id, new.name, new.email, new.fields);
END;

CREATE TRIGGER recipients_ad AFTER DELETE ON recipients BEGIN
  INSERT INTO recipient_fts(recipient_fts, rowid, name, email, fields) VALUES('delete', old.id, old.name, old.email, old.fields);
END;

CREATE TRIGGER recipients_au AFTER UPDATE ON recipients BEGIN
  INSERT INTO recipient_fts(recipient_fts, rowid, name, email, fields) VALUES('delete', old.id, old.name, old.email, old.fields);
  INSERT INTO recipient_fts(rowid, name, email, fields)
  VALUES (new.id, new.name, new.email, new.fields);
END;
