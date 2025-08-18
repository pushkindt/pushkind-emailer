use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::email::{
    EmailRecipient as DomainEmailRecipient, EmailWithRecipients as DomainEmailWithRecipients,
    NewEmail as DomainNewEmail, UpdateEmail as DomainUpdateEmail,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use crate::{
    models::{
        email::{
            Email as DbEmail, EmailRecipient as DbEmailRecipient, NewEmail as DbNewEmail,
            NewEmailRecipient as DbNewEmailRecipient,
        },
        recipient::Recipient as DbRecipient,
    },
    repository::{DieselRepository, EmailReader, EmailWriter},
};

impl EmailReader for DieselRepository {
    fn list_emails_not_replied_recipients(
        &self,
        hub_id: i32,
    ) -> RepositoryResult<Vec<DomainEmailRecipient>> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;

        let recipients = email_recipients::table
            .filter(email_recipients::replied.eq(false))
            .inner_join(emails::table)
            .filter(emails::hub_id.eq(hub_id))
            .select(DbEmailRecipient::as_select())
            .load::<DbEmailRecipient>(&mut conn)?;

        Ok(recipients.into_iter().map(Into::into).collect())
    }

    fn get_recipient(&self, id: i32) -> RepositoryResult<Option<DomainEmailRecipient>> {
        use crate::schema::email_recipients;
        let mut conn = self.conn()?;

        let recipient = email_recipients::table
            .filter(email_recipients::id.eq(id))
            .select(DbEmailRecipient::as_select())
            .first::<DbEmailRecipient>(&mut conn)
            .optional()?;

        Ok(recipient.map(Into::into))
    }

    fn get_email_by_id(&self, id: i32) -> RepositoryResult<Option<DomainEmailWithRecipients>> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.conn()?;

        let email = emails::table
            .filter(emails::id.eq(id))
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

    fn list_emails(&self, hub_id: i32) -> RepositoryResult<Vec<DomainEmailWithRecipients>> {
        use crate::schema::emails;
        let mut conn = self.conn()?;

        let db_emails: Vec<DbEmail> = emails::table
            .filter(emails::hub_id.eq(hub_id))
            .order(emails::created_at.desc())
            .select(DbEmail::as_select())
            .load(&mut conn)?;

        if db_emails.is_empty() {
            return Ok(Vec::new());
        }

        let db_recipients: Vec<DbEmailRecipient> = DbEmailRecipient::belonging_to(&db_emails)
            .select(DbEmailRecipient::as_select())
            .load(&mut conn)?;

        let grouped = db_recipients.grouped_by(&db_emails);

        Ok(db_emails
            .into_iter()
            .zip(grouped)
            .map(|(email, recipients)| DomainEmailWithRecipients {
                email: email.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            })
            .collect())
    }
}

impl EmailWriter for DieselRepository {
    fn create_email(&self, email: &DomainNewEmail) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::{email_recipients, emails, groups_recipients, recipients as rec};
        let mut conn = self.conn()?;

        conn.transaction::<_, RepositoryError, _>(|conn| {
            let created_at = chrono::Utc::now().naive_utc();
            let new_email: DbNewEmail = email.into();

            let inserted: DbEmail = diesel::insert_into(emails::table)
                .values(&new_email)
                .get_result(conn)?;

            for item in &email.recipients {
                if item.contains('@') {
                    let r: DbRecipient = rec::table
                        .filter(rec::email.eq(item.trim()))
                        .filter(rec::unsubscribed_at.is_null())
                        .select(DbRecipient::as_select())
                        .first(conn)?;

                    let new_rec = DbNewEmailRecipient {
                        email_id: inserted.id,
                        address: &r.email,
                        opened: false,
                        updated_at: created_at,
                        is_sent: false,
                        replied: false,
                        name: Some(&r.name),
                    };
                    diesel::insert_into(email_recipients::table)
                        .values(&new_rec)
                        .execute(conn)?;
                } else {
                    let group_id: i32 = item
                        .parse()
                        .map_err(|_| RepositoryError::ValidationError("invalid group id".into()))?;

                    let members: Vec<DbRecipient> = groups_recipients::table
                        .filter(groups_recipients::group_id.eq(group_id))
                        .inner_join(rec::table.on(groups_recipients::recipient_id.eq(rec::id)))
                        .select(DbRecipient::as_select())
                        .load(conn)?;

                    for member in members {
                        let new_rec = DbNewEmailRecipient {
                            email_id: inserted.id,
                            address: &member.email,
                            opened: false,
                            updated_at: created_at,
                            is_sent: false,
                            replied: false,
                            name: Some(&member.name),
                        };
                        diesel::insert_into(email_recipients::table)
                            .values(&new_rec)
                            .execute(conn)?;
                    }
                }
            }

            let recipients = email_recipients::table
                .filter(email_recipients::email_id.eq(inserted.id))
                .select(DbEmailRecipient::as_select())
                .load::<DbEmailRecipient>(conn)?;

            Ok(DomainEmailWithRecipients {
                email: inserted.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            })
        })
    }

    fn update_email(
        &self,
        email_id: i32,
        updates: &DomainUpdateEmail,
    ) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::emails;
        let mut conn = self.conn()?;
        diesel::update(emails::table.filter(emails::id.eq(email_id)))
            .set((
                emails::num_sent.eq(updates.num_sent),
                emails::num_opened.eq(updates.num_opened),
                emails::num_replied.eq(updates.num_replied),
            ))
            .execute(&mut conn)?;

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

        if let Some(is_sent) = updates.is_sent {
            diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
                .set((
                    email_recipients::is_sent.eq(is_sent),
                    email_recipients::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
        }
        if let Some(opened) = updates.opened {
            diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
                .set((
                    email_recipients::opened.eq(opened),
                    email_recipients::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
        }
        if let Some(replied) = updates.replied {
            diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
                .set((
                    email_recipients::replied.eq(replied),
                    email_recipients::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&mut conn)?;
        }

        // Recalculate num_opened, num_sent, num_replied for emails::table
        let num_sent = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::is_sent.eq(true))
            .count()
            .get_result::<i64>(&mut conn)? as i32;

        let num_opened = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::opened.eq(true))
            .count()
            .get_result::<i64>(&mut conn)? as i32;

        let num_replied = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::replied.eq(true))
            .count()
            .get_result::<i64>(&mut conn)? as i32;

        diesel::update(emails::table.filter(emails::id.eq(email_id)))
            .set((
                emails::num_sent.eq(num_sent),
                emails::num_opened.eq(num_opened),
                emails::num_replied.eq(num_replied),
            ))
            .execute(&mut conn)?;

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
