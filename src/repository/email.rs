use diesel::prelude::*;
use pushkind_common::domain::emailer::email::{
    EmailRecipient as DomainEmailRecipient, EmailWithRecipients as DomainEmailWithRecipients,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use pushkind_common::models::emailer::email::{
    Email as DbEmail, EmailRecipient as DbEmailRecipient,
    UpdateEmailRecipient as DbUpdateEmailRecipient,
};
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
        use pushkind_common::schema::emailer::{email_recipients, emails};
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
        use pushkind_common::schema::emailer::emails;
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
    fn list_recipients_grouped_by_address(
        &self,
        hub_id: i32,
    ) -> RepositoryResult<Vec<DomainEmailRecipient>> {
        use pushkind_common::schema::emailer::{email_recipients, emails};

        let mut conn = self.conn()?;

        let rows: Vec<(i32, DbEmailRecipient)> = email_recipients::table
            .inner_join(emails::table)
            .filter(emails::hub_id.eq(hub_id))
            .select((emails::hub_id, DbEmailRecipient::as_select()))
            .order((
                emails::hub_id.asc(),
                email_recipients::address.asc(),
                email_recipients::updated_at.desc(),
                email_recipients::id.desc(),
            ))
            .load(&mut conn)?;

        let mut latest: Vec<DbEmailRecipient> = Vec::new();
        let mut last_key: Option<(i32, String)> = None;

        for (row_hub_id, recipient) in rows {
            let is_same_group = last_key
                .as_ref()
                .map(|(hub, address)| *hub == row_hub_id && address == &recipient.address)
                .unwrap_or(false);

            if !is_same_group {
                last_key = Some((row_hub_id, recipient.address.clone()));
                latest.push(recipient);
            }
        }

        Ok(latest.into_iter().map(Into::into).collect())
    }
}

impl EmailWriter for DieselRepository {
    fn update_recipient(
        &self,
        recipient_id: i32,
        updates: &DomainUpdateEmailRecipient,
    ) -> RepositoryResult<DomainEmailWithRecipients> {
        use pushkind_common::schema::emailer::{email_recipients, emails};

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
        use pushkind_common::schema::emailer::{email_recipients, emails};
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
    use diesel::connection::SimpleConnection;
    use pushkind_common::db::establish_connection_pool;
    use tempfile::tempdir;

    #[test]
    fn list_recipients_grouped_by_address_returns_latest_snapshot() {
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
                    (1, 1, 'a@example.com', 0, '2024-01-01 10:00:00', 0, 0, NULL, 'Old A', '{"segment":"old"}'),
                    (2, 2, 'a@example.com', 1, '2024-01-02 10:00:00', 1, 1, 'Thanks', 'New A', '{"segment":"new"}'),
                    (3, 2, 'b@example.com', 0, '2024-01-01 11:00:00', 0, 0, NULL, 'Bee', '{"segment":"bee"}'),
                    (4, 3, 'a@example.com', 1, '2024-01-03 12:00:00', 1, 0, NULL, 'Hub2 A', '{"segment":"hub2"}');
                "#,
            )
            .unwrap();
        }

        let repo = DieselRepository::new(pool.clone());

        let recipients = repo.list_recipients_grouped_by_address(1).unwrap();
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

        let recipients_hub2 = repo.list_recipients_grouped_by_address(2).unwrap();
        assert_eq!(recipients_hub2.len(), 1);
        assert_eq!(recipients_hub2[0].address, "a@example.com");
        assert_eq!(recipients_hub2[0].name, "Hub2 A");
        assert_eq!(
            recipients_hub2[0].fields.get("segment").map(String::as_str),
            Some("hub2")
        );
    }
}
