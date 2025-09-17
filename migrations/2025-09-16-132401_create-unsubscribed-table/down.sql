-- This file should undo anything in `up.sql`
ALTER TABLE recipients
ADD COLUMN unsubscribed_at TIMESTAMP;

-- update recipients set unsubscribed_at = created_at from unsubscribes
-- where recipients.email = unsubscribes.email and recipients.hub_id = unsubscribes.hub_id
UPDATE recipients
SET unsubscribed_at = unsubscribes.created_at
FROM unsubscribes
WHERE recipients.email = unsubscribes.email
  AND recipients.hub_id = unsubscribes.hub_id;

-- DROP unsubscribes table
DROP TABLE unsubscribes;
