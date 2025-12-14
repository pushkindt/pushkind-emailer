use std::collections::HashSet;

use crate::domain::email::{
    EmailRecipient as DomainEmailRecipient, EmailWithRecipients as DomainEmailWithRecipients,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use crate::models::email::{
    Email as DbEmail, EmailRecipient as DbEmailRecipient,
    UpdateEmailRecipient as DbUpdateEmailRecipient,
};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use super::helpers::apply_pagination;
use crate::repository::{
    DieselRepository, EmailListQuery, EmailReader, EmailRecipientReader, EmailWriter,
};

impl EmailReader for DieselRepository {
    fn get_email_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<DomainEmailWithRecipients>> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;

        let email = emails::table
            .filter(emails::id.eq(id))
            .filter(emails::hub_id.eq(hub_id))
            .select(DbEmail::as_select())
            .first::<DbEmail>(&mut conn)
            .optional()?;

        if let Some(email) = email {
            let recipients = email_recipients::table
                .filter(email_recipients::email_id.eq(email.id))
                .select(DbEmailRecipient::as_select())
                .load::<DbEmailRecipient>(&mut conn)?;

            Ok(Some(DomainEmailWithRecipients {
                email: email.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            }))
        } else {
            Ok(None)
        }
    }

    fn list_emails(
        &self,
        query: EmailListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainEmailWithRecipients>)> {
        use crate::schema::emails;
        let mut conn = self.conn()?;

        let query_builder = || {
            emails::table
                .filter(emails::hub_id.eq(query.hub_id))
                .select(DbEmail::as_select())
                .into_boxed::<diesel::sqlite::Sqlite>()
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();
        items = apply_pagination(items, query.pagination.as_ref());

        let db_emails = items
            .order(emails::created_at.desc())
            .load::<DbEmail>(&mut conn)?;

        if db_emails.is_empty() {
            return Ok((total, vec![]));
        }

        let db_recipients: Vec<DbEmailRecipient> = DbEmailRecipient::belonging_to(&db_emails)
            .select(DbEmailRecipient::as_select())
            .load(&mut conn)?;

        let grouped = db_recipients.grouped_by(&db_emails);

        let result: Vec<DomainEmailWithRecipients> = db_emails
            .into_iter()
            .zip(grouped)
            .map(|(email, recipients)| DomainEmailWithRecipients {
                email: email.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            })
            .collect();

        Ok((total, result))
    }
}

impl EmailRecipientReader for DieselRepository {
    fn list_recent_recipients(
        &self,
        hub_id: i32,
        // Only include recipients whose most recent email was sent strictly
        // after `number_of_days` ago. `None` skips filtering.
        number_of_days: Option<i64>,
    ) -> RepositoryResult<Vec<DomainEmailRecipient>> {
        use crate::schema::{email_recipients, emails};

        let mut conn = self.conn()?;

        // Build the base query (SQLite-compatible)
        let mut query = email_recipients::table
            .inner_join(emails::table)
            .filter(emails::hub_id.eq(hub_id))
            .into_boxed();

        // Push the created_at cutoff into the DB
        if let Some(days) = number_of_days.filter(|d| *d > 0) {
            let cutoff = Utc::now().naive_utc() - Duration::days(days);
            // keep emails strictly AFTER the cutoff
            query = query.filter(emails::created_at.gt(cutoff));
        }

        // Step 1: Load all rows sorted so newest per address comes first
        let rows: Vec<(DbEmailRecipient, NaiveDateTime)> = query
            .order((
                email_recipients::address.asc(),
                emails::created_at.desc(),
                email_recipients::updated_at.desc(),
            ))
            .select((DbEmailRecipient::as_select(), emails::created_at))
            .load(&mut conn)?;

        // Step 2: Keep only the first row per address (Rust-side dedup)
        let mut seen = HashSet::new();
        let mut latest = Vec::with_capacity(rows.len());
        for (recipient, email_created_at) in rows {
            if seen.insert(recipient.address.clone()) {
                latest.push((recipient, email_created_at));
            }
        }

        // Step 3: Sort the final set by the originating email creation time
        latest.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.0.updated_at.cmp(&a.0.updated_at))
        });

        Ok(latest
            .into_iter()
            .map(|(recipient, _)| recipient.into())
            .collect())
    }
}

impl EmailWriter for DieselRepository {
    fn update_recipient(
        &self,
        recipient_id: i32,
        updates: &DomainUpdateEmailRecipient,
    ) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::{email_recipients, emails};

        let mut conn = self.conn()?;
        let email_id: i32 = email_recipients::table
            .filter(email_recipients::id.eq(recipient_id))
            .select(email_recipients::email_id)
            .first(&mut conn)?;

        let changeset = DbUpdateEmailRecipient::from(updates);
        diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
            .set(changeset)
            .execute(&mut conn)?;

        DbEmail::recalc_email_stats(&mut conn, email_id)?;

        let email = emails::table
            .filter(emails::id.eq(email_id))
            .select(DbEmail::as_select())
            .first::<DbEmail>(&mut conn)?;

        let recipients = DbEmailRecipient::belonging_to(&email)
            .select(DbEmailRecipient::as_select())
            .load::<DbEmailRecipient>(&mut conn)?;

        Ok(DomainEmailWithRecipients {
            email: email.into(),
            recipients: recipients.into_iter().map(Into::into).collect(),
        })
    }

    fn delete_email(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;
        diesel::delete(email_recipients::table.filter(email_recipients::email_id.eq(id)))
            .execute(&mut conn)?;
        diesel::delete(emails::table.filter(emails::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::repository::{DieselRepository, EmailRecipientReader};
    use chrono::{Duration, Utc};
    use diesel::connection::SimpleConnection;
    use pushkind_common::db::{DbPool, establish_connection_pool};
    use tempfile::{TempDir, tempdir};

    fn setup_db() -> (TempDir, DbPool) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.sqlite");
        let db_path = db_path.to_string_lossy().to_string();
        let pool = establish_connection_pool(&db_path).unwrap();

        {
            let mut conn = pool.get().unwrap();

            conn.batch_execute(
                r#"
                CREATE TABLE hubs (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL
                );
                "#,
            )
            .unwrap();

            conn.batch_execute(
                r#"
                CREATE TABLE emails (
                    id INTEGER PRIMARY KEY,
                    message TEXT NOT NULL,
                    created_at TIMESTAMP NOT NULL,
                    is_sent BOOLEAN NOT NULL,
                    subject TEXT,
                    attachment BLOB,
                    attachment_name TEXT,
                    attachment_mime TEXT,
                    num_sent INTEGER NOT NULL,
                    num_opened INTEGER NOT NULL,
                    num_replied INTEGER NOT NULL,
                    hub_id INTEGER NOT NULL REFERENCES hubs(id)
                );
                "#,
            )
            .unwrap();

            conn.batch_execute(
                r#"
                CREATE TABLE email_recipients (
                    id INTEGER PRIMARY KEY,
                    email_id INTEGER NOT NULL REFERENCES emails(id),
                    address TEXT NOT NULL,
                    opened BOOLEAN NOT NULL,
                    updated_at TIMESTAMP NOT NULL,
                    is_sent BOOLEAN NOT NULL,
                    replied BOOLEAN NOT NULL,
                    reply TEXT,
                    name TEXT NOT NULL,
                    fields TEXT NOT NULL
                );
                "#,
            )
            .unwrap();
        }

        (dir, pool)
    }

    #[test]
    fn list_recent_recipients_returns_latest_snapshot() {
        let (_dir, pool) = setup_db();

        {
            let mut conn = pool.get().unwrap();

            conn.batch_execute(
                r#"
                INSERT INTO hubs (id, name) VALUES (1, 'Hub 1'), (2, 'Hub 2');
                "#,
            )
            .unwrap();

            conn.batch_execute(
                r#"
                INSERT INTO emails (
                    id,
                    message,
                    created_at,
                    is_sent,
                    subject,
                    attachment,
                    attachment_name,
                    attachment_mime,
                    num_sent,
                    num_opened,
                    num_replied,
                    hub_id
                )
                VALUES
                    (1, 'Msg', '2024-01-01 00:00:00', 0, NULL, NULL, NULL, NULL, 0, 0, 0, 1),
                    (2, 'Msg', '2024-01-02 00:00:00', 0, NULL, NULL, NULL, NULL, 0, 0, 0, 1),
                    (3, 'Msg', '2024-01-03 00:00:00', 0, NULL, NULL, NULL, NULL, 0, 0, 0, 2);
                "#,
            )
            .unwrap();

            conn.batch_execute(
                r#"
                INSERT INTO email_recipients (
                    id,
                    email_id,
                    address,
                    opened,
                    updated_at,
                    is_sent,
                    replied,
                    reply,
                    name,
                    fields
                )
                VALUES
                    (1, 1, 'a@example.com', 0, '2024-01-04 10:00:00', 0, 0, NULL, 'Old A', '{"segment":"old"}'),
                    (2, 2, 'a@example.com', 1, '2024-01-02 10:00:00', 1, 1, 'Thanks', 'New A', '{"segment":"new"}'),
                    (3, 2, 'b@example.com', 0, '2024-01-01 11:00:00', 0, 0, NULL, 'Bee', '{"segment":"bee"}'),
                    (4, 3, 'a@example.com', 1, '2024-01-03 12:00:00', 1, 0, NULL, 'Hub2 A', '{"segment":"hub2"}');
                "#,
            )
            .unwrap();
        }

        let repo = DieselRepository::new(pool.clone());

        let recipients = repo.list_recent_recipients(1, None).unwrap();
        assert_eq!(recipients.len(), 2);

        let recipient_a = recipients
            .iter()
            .find(|recipient| recipient.address == "a@example.com")
            .unwrap();
        assert_eq!(recipient_a.email_id, 2);
        assert!(recipient_a.opened);
        assert!(recipient_a.is_sent);
        assert!(recipient_a.replied);
        assert_eq!(recipient_a.reply.as_deref(), Some("Thanks"));
        assert_eq!(recipient_a.name, "New A");
        assert_eq!(
            recipient_a.fields.get("segment").map(String::as_str),
            Some("new")
        );

        let recipient_b = recipients
            .iter()
            .find(|recipient| recipient.address == "b@example.com")
            .unwrap();
        assert_eq!(recipient_b.email_id, 2);
        assert_eq!(recipient_b.name, "Bee");
        assert_eq!(
            recipient_b.fields.get("segment").map(String::as_str),
            Some("bee")
        );

        let recipients_hub2 = repo.list_recent_recipients(2, None).unwrap();
        assert_eq!(recipients_hub2.len(), 1);
        assert_eq!(recipients_hub2[0].address, "a@example.com");
        assert_eq!(recipients_hub2[0].name, "Hub2 A");
        assert_eq!(
            recipients_hub2[0].fields.get("segment").map(String::as_str),
            Some("hub2")
        );
    }

    #[test]
    fn list_recent_recipients_applies_number_of_days_filter() {
        let (_dir, pool) = setup_db();

        let now = Utc::now().naive_utc();
        let recent = now - Duration::hours(12);
        let old = now - Duration::days(5);

        {
            let mut conn = pool.get().unwrap();

            conn.batch_execute(
                r#"
                INSERT INTO hubs (id, name) VALUES (1, 'Hub 1');
                "#,
            )
            .unwrap();

            let insert_emails = format!(
                "INSERT INTO emails (id, message, created_at, is_sent, subject, attachment, attachment_name, attachment_mime, num_sent, num_opened, num_replied, hub_id) VALUES
                    (1, 'Msg', '{old}', 1, NULL, NULL, NULL, NULL, 0, 0, 0, 1),
                    (2, 'Msg', '{recent}', 1, NULL, NULL, NULL, NULL, 0, 0, 0, 1);",
            );
            conn.batch_execute(&insert_emails).unwrap();

            let insert_recipients = format!(
                "INSERT INTO email_recipients (id, email_id, address, opened, updated_at, is_sent, replied, reply, name, fields) VALUES
                    (1, 1, 'slow@example.com', 0, '{old}', 1, 0, NULL, 'Slow', '{{}}'),
                    (2, 2, 'fast@example.com', 0, '{recent}', 1, 0, NULL, 'Fast', '{{}}');",
            );
            conn.batch_execute(&insert_recipients).unwrap();
        }

        let repo = DieselRepository::new(pool.clone());

        let all = repo.list_recent_recipients(1, None).unwrap();
        assert_eq!(all.len(), 2);

        let filtered = repo.list_recent_recipients(1, Some(3)).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].address, "fast@example.com");
    }
}
