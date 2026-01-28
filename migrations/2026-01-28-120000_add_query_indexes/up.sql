-- Add indexes to support common query patterns and joins
CREATE INDEX idx_emails_hub_created_at
    ON emails (hub_id, created_at);

CREATE INDEX idx_recipients_hub_name
    ON recipients (hub_id, name);

CREATE INDEX idx_groups_hub_name
    ON groups (hub_id, name);

CREATE INDEX idx_groups_recipients_recipient_id
    ON groups_recipients (recipient_id);

CREATE INDEX idx_unsubscribes_hub_email
    ON unsubscribes (hub_id, email);

CREATE INDEX idx_unsubscribes_hub_created_at
    ON unsubscribes (hub_id, created_at);
