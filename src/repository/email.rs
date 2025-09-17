use diesel::prelude::*;
use pushkind_common::domain::emailer::email::{
    EmailWithRecipients as DomainEmailWithRecipients,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use pushkind_common::models::emailer::email::{
    Email as DbEmail, EmailRecipient as DbEmailRecipient,
    UpdateEmailRecipient as DbUpdateEmailRecipient,
};
use pushkind_common::repository::errors::RepositoryResult;

use super::helpers::apply_pagination;
use crate::repository::EmailListQuery;
use crate::repository::{DieselRepository, EmailReader, EmailWriter};

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
