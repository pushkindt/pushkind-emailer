//! Repository operations for emails and email recipients.
use std::collections::HashSet;

use crate::domain::email::{
    EmailRecipient as DomainEmailRecipient, EmailWithRecipients as DomainEmailWithRecipients,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use crate::domain::types::{EmailId, HubId, RecipientId};
use crate::models::email::{
    Email as DbEmail, EmailRecipient as DbEmailRecipient,
    UpdateEmailRecipient as DbUpdateEmailRecipient,
};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use super::helpers::apply_pagination;
use crate::repository::{DieselRepository, EmailListQuery, EmailReader, EmailWriter};

impl EmailReader for DieselRepository {
    fn get_email_by_id(
        &self,
        id: EmailId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<DomainEmailWithRecipients>> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;

        conn.transaction::<Option<DomainEmailWithRecipients>, RepositoryError, _>(|conn| {
            let email = emails::table
                .filter(emails::id.eq(id.get()))
                .filter(emails::hub_id.eq(hub_id.get()))
                .select(DbEmail::as_select())
                .first::<DbEmail>(conn)
                .optional()?;

            let Some(email) = email else {
                return Ok(None);
            };

            let recipients = email_recipients::table
                .filter(email_recipients::email_id.eq(email.id))
                .select(DbEmailRecipient::as_select())
                .load::<DbEmailRecipient>(conn)?;

            let email = DomainEmailWithRecipients {
                email: email.try_into()?,
                recipients: recipients
                    .into_iter()
                    .map(|recipient| recipient.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
            };

            Ok(Some(email))
        })
    }

    fn list_emails(
        &self,
        query: EmailListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainEmailWithRecipients>)> {
        use crate::schema::emails;
        let mut conn = self.conn()?;

        conn.transaction::<(usize, Vec<DomainEmailWithRecipients>), RepositoryError, _>(|conn| {
            let query_builder = || {
                emails::table
                    .filter(emails::hub_id.eq(query.hub_id.get()))
                    .select(DbEmail::as_select())
                    .into_boxed::<diesel::sqlite::Sqlite>()
            };

            let total = query_builder().count().get_result::<i64>(conn)? as usize;

            let mut items = query_builder();
            items = apply_pagination(items, query.pagination.as_ref());

            let db_emails = items
                .order(emails::created_at.desc())
                .load::<DbEmail>(conn)?;

            if db_emails.is_empty() {
                return Ok((total, vec![]));
            }

            let db_recipients: Vec<DbEmailRecipient> = DbEmailRecipient::belonging_to(&db_emails)
                .select(DbEmailRecipient::as_select())
                .load(conn)?;

            let grouped = db_recipients.grouped_by(&db_emails);

            let result: Vec<DomainEmailWithRecipients> = db_emails
                .into_iter()
                .zip(grouped)
                .map(|(email, recipients)| {
                    Ok::<_, RepositoryError>(DomainEmailWithRecipients {
                        email: email.try_into()?,
                        recipients: recipients
                            .into_iter()
                            .map(|recipient| recipient.try_into())
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok((total, result))
        })
    }

    fn list_recent_email_recipients(
        &self,
        hub_id: HubId,
        // Only include recipients whose most recent email was sent strictly
        // after `number_of_days` ago. `None` skips filtering.
        number_of_days: Option<i64>,
    ) -> RepositoryResult<Vec<DomainEmailRecipient>> {
        use crate::schema::{email_recipients, emails};

        let mut conn = self.conn()?;

        // Build the base query (SQLite-compatible)
        let mut query = email_recipients::table
            .inner_join(emails::table)
            .filter(emails::hub_id.eq(hub_id.get()))
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
        let mut seen: HashSet<String> = HashSet::new();
        let mut latest = Vec::with_capacity(rows.len());
        for (recipient, email_created_at) in rows {
            if seen.insert(recipient.address.trim().to_lowercase()) {
                latest.push((recipient, email_created_at));
            }
        }

        // Step 3: Sort the final set by the originating email creation time
        latest.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.0.updated_at.cmp(&a.0.updated_at))
        });

        let recipients = latest
            .into_iter()
            .map(|(recipient, _)| recipient.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(recipients)
    }
}

impl EmailWriter for DieselRepository {
    fn update_email_recipient(
        &self,
        recipient_id: RecipientId,
        updates: &DomainUpdateEmailRecipient,
    ) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::{email_recipients, emails};

        let mut conn = self.conn()?;
        conn.transaction::<DomainEmailWithRecipients, RepositoryError, _>(|conn| {
            let email_id: i32 = email_recipients::table
                .filter(email_recipients::id.eq(recipient_id.get()))
                .select(email_recipients::email_id)
                .first(conn)?;

            let changeset = DbUpdateEmailRecipient::from(updates);
            diesel::update(
                email_recipients::table.filter(email_recipients::id.eq(recipient_id.get())),
            )
            .set(changeset)
            .execute(conn)?;

            DbEmail::recalc_email_stats(conn, email_id)?;

            let email = emails::table
                .filter(emails::id.eq(email_id))
                .select(DbEmail::as_select())
                .first::<DbEmail>(conn)?;

            let recipients = DbEmailRecipient::belonging_to(&email)
                .select(DbEmailRecipient::as_select())
                .load::<DbEmailRecipient>(conn)?;

            Ok(DomainEmailWithRecipients {
                email: email.try_into()?,
                recipients: recipients
                    .into_iter()
                    .map(|recipient| recipient.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
    }

    fn delete_email(&self, id: EmailId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;
        conn.transaction::<(), RepositoryError, _>(|conn| {
            let email_exists = emails::table
                .filter(emails::id.eq(id.get()))
                .filter(emails::hub_id.eq(hub_id.get()))
                .select(emails::id)
                .first::<i32>(conn)
                .optional()?;
            if email_exists.is_none() {
                return Err(RepositoryError::NotFound);
            }

            diesel::delete(email_recipients::table.filter(email_recipients::email_id.eq(id.get())))
                .execute(conn)?;
            let deleted = diesel::delete(
                emails::table
                    .filter(emails::id.eq(id.get()))
                    .filter(emails::hub_id.eq(hub_id.get())),
            )
            .execute(conn)?;
            if deleted == 0 {
                return Err(RepositoryError::NotFound);
            }
            Ok(())
        })
    }
}
