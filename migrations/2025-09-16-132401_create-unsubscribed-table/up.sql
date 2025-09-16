-- Your SQL goes here
CREATE TABLE unsubscribes (
    email VARCHAR(255) NOT NULL,
    hub_id INTEGER NOT NULL,
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (email, hub_id)
);

INSERT INTO unsubscribes (email, hub_id, created_at)
SELECT email, hub_id, unsubscribed_at AS created_at
FROM recipients
WHERE unsubscribed_at IS NOT NULL;

ALTER TABLE recipients
DROP COLUMN unsubscribed_at;
