use pushkind_common::db::DbPool;
use std::error::Error;

use diesel::prelude::*;

use crate::repository::errors::RepositoryResult;
use crate::{
    models::{
        email::{Email, EmailRecipient, NewEmail, NewEmailRecipient},
        recipient::Recipient,
    },
    repository::{EmailReader, EmailWriter},
};

/// Diesel implementation of [`EmailRepository`].
pub struct DieselEmailRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselEmailRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

use crate::domain::email::{
    EmailWithRecipients as DomainEmailWithRecipients, NewEmail as DomainNewEmail,
    UpdateEmail as DomainUpdateEmail, UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use crate::repository::errors::RepositoryError;

impl EmailReader for DieselEmailRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<DomainEmailWithRecipients>> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.pool.get()?;

        let email = emails::table
            .filter(emails::id.eq(id))
            .select(Email::as_select())
            .first::<Email>(&mut conn)
            .optional()?;

        if let Some(email) = email {
            let recipients = email_recipients::table
                .filter(email_recipients::email_id.eq(email.id))
                .select(EmailRecipient::as_select())
                .load::<EmailRecipient>(&mut conn)?;

            Ok(Some(DomainEmailWithRecipients {
                email: email.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            }))
        } else {
            Ok(None)
        }
    }

    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<DomainEmailWithRecipients>> {
        use crate::schema::emails;
        let mut conn = self.pool.get()?;

        let db_emails: Vec<Email> = emails::table
            .filter(emails::hub_id.eq(hub_id))
            .order(emails::created_at.desc())
            .select(Email::as_select())
            .load(&mut conn)?;

        if db_emails.is_empty() {
            return Ok(Vec::new());
        }

        let db_recipients: Vec<EmailRecipient> = EmailRecipient::belonging_to(&db_emails)
            .select(EmailRecipient::as_select())
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

impl EmailWriter for DieselEmailRepository<'_> {
    fn create(&self, email: &DomainNewEmail) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::{email_recipients, emails, groups_recipients, recipients as rec};
        let mut conn = self.pool.get()?;

        conn.transaction::<_, RepositoryError, _>(|conn| {
            let created_at = chrono::Utc::now().naive_utc();
            let new_email = NewEmail {
                message: email.message,
                created_at: &created_at,
                is_sent: false,
                subject: email.subject,
                attachment: email.attachment,
                attachment_name: email.attachment_name,
                attachment_mime: email.attachment_mime,
                hub_id: email.hub_id,
            };

            let inserted: Email = diesel::insert_into(emails::table)
                .values(&new_email)
                .get_result(conn)?;

            for item in &email.recipients {
                if item.contains('@') {
                    let r: Recipient = rec::table
                        .filter(rec::email.eq(item.trim()))
                        .filter(rec::unsubscribed_at.is_null())
                        .select(Recipient::as_select())
                        .first(conn)?;

                    let new_rec = NewEmailRecipient {
                        email_id: inserted.id,
                        address: &r.email,
                        opened: false,
                        updated_at: &created_at,
                        is_sent: false,
                        replied: false,
                    };
                    diesel::insert_into(email_recipients::table)
                        .values(&new_rec)
                        .execute(conn)?;
                } else {
                    let group_id: i32 = item
                        .parse()
                        .map_err(|_| RepositoryError::ValidationError("invalid group id".into()))?;

                    let members: Vec<Recipient> = groups_recipients::table
                        .filter(groups_recipients::group_id.eq(group_id))
                        .inner_join(rec::table.on(groups_recipients::recipient_id.eq(rec::id)))
                        .select(Recipient::as_select())
                        .load(conn)?;

                    for member in members {
                        let new_rec = NewEmailRecipient {
                            email_id: inserted.id,
                            address: &member.email,
                            opened: false,
                            updated_at: &created_at,
                            is_sent: false,
                            replied: false,
                        };
                        diesel::insert_into(email_recipients::table)
                            .values(&new_rec)
                            .execute(conn)?;
                    }
                }
            }

            let recipients = email_recipients::table
                .filter(email_recipients::email_id.eq(inserted.id))
                .select(EmailRecipient::as_select())
                .load::<EmailRecipient>(conn)?;

            Ok(DomainEmailWithRecipients {
                email: inserted.into(),
                recipients: recipients.into_iter().map(Into::into).collect(),
            })
        })
    }

    fn update(
        &self,
        email_id: i32,
        updates: &DomainUpdateEmail,
    ) -> RepositoryResult<DomainEmailWithRecipients> {
        use crate::schema::emails::dsl as emails;
        let mut conn = self.pool.get()?;
        diesel::update(emails::emails.filter(emails::id.eq(email_id)))
            .set((
                emails::num_sent.eq(updates.num_sent),
                emails::num_opened.eq(updates.num_opened),
                emails::num_replied.eq(updates.num_replied),
            ))
            .execute(&mut conn)?;

        let email = emails::emails
            .filter(emails::id.eq(email_id))
            .select(Email::as_select())
            .first::<Email>(&mut conn)?;

        let recipients = EmailRecipient::belonging_to(&email)
            .select(EmailRecipient::as_select())
            .load::<EmailRecipient>(&mut conn)?;

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
        use crate::schema::email_recipients::dsl as er;
        let mut conn = self.pool.get()?;
        let email_id: i32 = er::email_recipients
            .filter(er::id.eq(recipient_id))
            .select(er::email_id)
            .first(&mut conn)?;

        diesel::update(er::email_recipients.filter(er::id.eq(recipient_id)))
            .set((
                er::opened.eq(updates.opened),
                er::is_sent.eq(updates.is_sent),
                er::replied.eq(updates.replied),
                er::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(&mut conn)?;

        use crate::schema::emails::dsl as emails;

        let email = emails::emails
            .filter(emails::id.eq(email_id))
            .select(Email::as_select())
            .first::<Email>(&mut conn)?;

        let recipients = EmailRecipient::belonging_to(&email)
            .select(EmailRecipient::as_select())
            .load::<EmailRecipient>(&mut conn)?;

        Ok(DomainEmailWithRecipients {
            email: email.into(),
            recipients: recipients.into_iter().map(Into::into).collect(),
        })
    }

    fn delete(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::{email_recipients, emails};
        let mut conn = self.pool.get()?;
        diesel::delete(email_recipients::table.filter(email_recipients::email_id.eq(id)))
            .execute(&mut conn)?;
        diesel::delete(emails::table.filter(emails::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }
}

pub fn get_hub_all_emails_with_recipients(
    conn: &mut SqliteConnection,
    hub_id: i32,
) -> QueryResult<Vec<(Email, Vec<EmailRecipient>)>> {
    use crate::schema::emails;

    // Read all emails for a user sorted by timestamp
    let all_emails: Vec<Email> = emails::table
        .filter(emails::hub_id.eq(hub_id))
        .order(emails::created_at.desc())
        .select(Email::as_select()) // Ensure Diesel knows we're selecting the full Email struct
        .load(conn)?;

    // Load all recipients belonging to the fetched emails
    let email_recipients: Vec<EmailRecipient> = EmailRecipient::belonging_to(&all_emails)
        .select(EmailRecipient::as_select()) // Ensure Diesel knows we're selecting the full EmailRecipient struct
        .load(conn)?;

    // Group recipients by email and return
    Ok(email_recipients
        .grouped_by(&all_emails)
        .into_iter()
        .zip(all_emails)
        .map(|(recipients, email)| (email, recipients))
        .collect())
}

fn create_email_recipient(
    conn: &mut SqliteConnection,
    email_id: i32,
    address: &str,
    updated_at: &chrono::NaiveDateTime,
) -> QueryResult<EmailRecipient> {
    use crate::schema::email_recipients;

    let new_email_recipient = NewEmailRecipient {
        email_id,
        address,
        opened: false,
        updated_at,
        is_sent: false,
        replied: false,
    };

    diesel::insert_into(email_recipients::table)
        .values(&new_email_recipient)
        .execute(conn)?;

    email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::address.eq(address))
        .first(conn)
}

pub fn create_email(
    conn: &mut SqliteConnection,
    subject: Option<&str>,
    message: &str,
    recipients: &Vec<String>,
    attachment: Option<&[u8]>,
    attachment_name: Option<&str>,
    attachment_mime: Option<&str>,
    hub_id: i32,
) -> Result<Email, Box<dyn Error>> {
    use crate::schema::emails;
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    let created_at = chrono::Utc::now().naive_utc();

    let new_email = NewEmail {
        hub_id,
        message,
        created_at: &created_at,
        is_sent: false,
        subject,
        attachment,
        attachment_name,
        attachment_mime,
    };

    diesel::insert_into(emails::table)
        .values(&new_email)
        .execute(conn)?;

    let email: Email = emails::table
        .filter(emails::hub_id.eq(hub_id))
        .filter(emails::created_at.eq(created_at))
        .filter(emails::message.eq(&new_email.message))
        .order(emails::created_at.desc())
        .first(conn)?;

    for recipient in recipients {
        // if recipient is an email and exists in the database create a new EmailRecipient
        // if recipient is not an email but a group id then fetch the group and create a new EmailRecipient for each member
        if recipient.contains('@') {
            let recipient = recipient.trim();
            let recipient: Recipient = recipients::table
                .filter(recipients::email.eq(recipient))
                .filter(recipients::unsubscribed_at.is_null())
                .select(Recipient::as_select())
                .first(conn)?;

            create_email_recipient(conn, email.id, &recipient.email, &created_at)?;
        } else {
            let group_id = recipient.parse::<i32>()?;

            let group_members: Vec<Recipient> = groups_recipients::table
                .filter(groups_recipients::group_id.eq(group_id))
                .inner_join(
                    recipients::table.on(groups_recipients::recipient_id.eq(recipients::id)),
                )
                .select(Recipient::as_select())
                .load(conn)?;

            for member in group_members {
                create_email_recipient(conn, email.id, &member.email, &created_at)?;
            }
        }
    }

    Ok(email)
}

pub fn remove_email(conn: &mut SqliteConnection, email_id: i32, hub_id: i32) -> QueryResult<usize> {
    use crate::schema::{email_recipients, emails};

    diesel::delete(
        emails::table
            .filter(emails::id.eq(email_id))
            .filter(emails::hub_id.eq(hub_id)),
    )
    .execute(conn)?;
    diesel::delete(email_recipients::table.filter(email_recipients::email_id.eq(email_id)))
        .execute(conn)
}

pub fn get_email(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<Email> {
    use crate::schema::emails;

    emails::table.filter(emails::id.eq(email_id)).first(conn)
}

pub fn get_email_recipients(
    conn: &mut SqliteConnection,
    email_id: i32,
) -> QueryResult<Vec<EmailRecipient>> {
    use crate::schema::email_recipients;

    email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .load(conn)
}

pub fn set_email_sent_status(
    conn: &mut SqliteConnection,
    email_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::emails;

    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::is_sent.eq(status))
        .execute(conn)
}

pub fn set_email_recipient_sent_status(
    conn: &mut SqliteConnection,
    recipient_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set(email_recipients::is_sent.eq(status))
        .execute(conn)
}

pub fn set_email_recipient_opened_status(
    conn: &mut SqliteConnection,
    recipient_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set(email_recipients::opened.eq(status))
        .execute(conn)
}

pub fn reset_email_sent_and_opened_status(
    conn: &mut SqliteConnection,
    email_id: i32,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    set_email_sent_status(conn, email_id, false)?;

    diesel::update(email_recipients::table.filter(email_recipients::email_id.eq(email_id)))
        .set((
            email_recipients::opened.eq(false),
            email_recipients::is_sent.eq(false),
        ))
        .execute(conn)
}

pub fn get_hub_email_recipients_not_replied(
    conn: &mut SqliteConnection,
    hub_id: i32,
) -> QueryResult<Vec<EmailRecipient>> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    email_recipients::table
        .inner_join(emails::table.on(email_recipients::email_id.eq(emails::id)))
        .filter(emails::hub_id.eq(hub_id))
        .filter(email_recipients::replied.eq(false))
        .select(EmailRecipient::as_select())
        .load(conn)
}

pub fn set_email_recipient_replied_status(
    conn: &mut SqliteConnection,
    email_id: i32,
    recipient_id: i32,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set((
            email_recipients::replied.eq(true),
            email_recipients::is_sent.eq(true),
            email_recipients::opened.eq(true),
        ))
        .execute(conn)?;

    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::is_sent.eq(true))
        .execute(conn)
}

pub fn update_email_num_sent(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::is_sent.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_sent.eq(num_value as i32))
        .execute(conn)
}

pub fn update_email_num_opened(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::opened.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_opened.eq(num_value as i32))
        .execute(conn)
}

pub fn update_email_num_replied(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::replied.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_replied.eq(num_value as i32))
        .execute(conn)
}

pub fn get_email_recipient(
    conn: &mut SqliteConnection,
    recipient_id: i32,
) -> QueryResult<EmailRecipient> {
    use crate::schema::email_recipients;

    email_recipients::table
        .filter(email_recipients::id.eq(recipient_id))
        .first(conn)
}
