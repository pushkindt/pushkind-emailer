use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use pushkind_common::db::DbConnection;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::email::{
    Email as DomainEmail, EmailRecipient as DomainEmailRecipient, NewEmail as DomainNewEmail,
    UpdateEmailRecipient as DomainUpdateEmailRecipient,
};
use crate::domain::types::TypeConstraintError;
use crate::models::hub::Hub;

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations, QueryableByName)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(table_name = crate::schema::emails)]
#[diesel(foreign_derive)]
pub struct Email {
    pub id: i32,
    pub message: String,
    pub created_at: NaiveDateTime,
    pub is_sent: bool,
    pub subject: Option<String>,
    pub attachment: Option<Vec<u8>>,
    pub attachment_name: Option<String>,
    pub attachment_mime: Option<String>,
    pub num_sent: i32,
    pub num_opened: i32,
    pub num_replied: i32,
    pub hub_id: i32,
}

impl Email {
    pub fn recalc_email_stats(conn: &mut DbConnection, email_id: i32) -> RepositoryResult<()> {
        use crate::schema::{email_recipients, emails};

        let num_sent = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::is_sent.eq(true))
            .count()
            .get_result::<i64>(conn)? as i32;

        let num_opened = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::opened.eq(true))
            .count()
            .get_result::<i64>(conn)? as i32;

        let num_replied = email_recipients::table
            .filter(email_recipients::email_id.eq(email_id))
            .filter(email_recipients::replied.eq(true))
            .count()
            .get_result::<i64>(conn)? as i32;

        diesel::update(emails::table.filter(emails::id.eq(email_id)))
            .set((
                emails::num_sent.eq(num_sent),
                emails::num_opened.eq(num_opened),
                emails::num_replied.eq(num_replied),
            ))
            .execute(conn)?;

        Ok(())
    }
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::emails)]
pub struct NewEmail<'a> {
    pub message: &'a str,
    pub created_at: NaiveDateTime,
    pub is_sent: bool,
    pub subject: Option<&'a str>,
    pub attachment: Option<&'a [u8]>,
    pub attachment_name: Option<&'a str>,
    pub attachment_mime: Option<&'a str>,
    pub hub_id: i32,
}

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(Email, foreign_key = email_id))]
#[diesel(table_name = crate::schema::email_recipients)]
pub struct EmailRecipient {
    pub id: i32,
    pub email_id: i32,
    pub address: String,
    pub opened: bool,
    pub updated_at: NaiveDateTime,
    pub is_sent: bool,
    pub replied: bool,
    pub reply: Option<String>,
    pub name: String,
    pub fields: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::email_recipients)]
pub struct NewEmailRecipient<'a> {
    pub email_id: i32,
    pub address: &'a str,
    pub opened: bool,
    pub updated_at: NaiveDateTime,
    pub is_sent: bool,
    pub replied: bool,
    pub name: &'a str,
    pub fields: &'a str,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::email_recipients)]
pub struct UpdateEmailRecipient<'a> {
    opened: Option<bool>,
    is_sent: Option<bool>,
    replied: Option<bool>,
    reply: Option<&'a str>,
    updated_at: Option<NaiveDateTime>,
}

impl TryFrom<Email> for DomainEmail {
    type Error = TypeConstraintError;

    fn try_from(value: Email) -> Result<Self, Self::Error> {
        DomainEmail::try_new(
            value.id,
            value.message,
            value.created_at,
            value.is_sent,
            value.subject,
            value.attachment,
            value.attachment_name,
            value.attachment_mime,
            value.num_sent,
            value.num_opened,
            value.num_replied,
            value.hub_id,
        )
    }
}

impl TryFrom<EmailRecipient> for DomainEmailRecipient {
    type Error = TypeConstraintError;

    fn try_from(value: EmailRecipient) -> Result<Self, Self::Error> {
        DomainEmailRecipient::try_new(
            value.id,
            value.email_id,
            value.address,
            value.opened,
            value.updated_at,
            value.is_sent,
            value.replied,
            value.reply,
            value.name,
            serde_json::from_str(&value.fields).unwrap_or_default(),
        )
    }
}

impl<'a> From<&'a DomainNewEmail> for NewEmail<'a> {
    fn from(value: &'a DomainNewEmail) -> Self {
        Self {
            message: value.message.as_str(),
            created_at: Utc::now().naive_utc(),
            is_sent: false,
            subject: value.subject.as_ref().map(|subject| subject.as_str()),
            attachment: value.attachment.as_deref(),
            attachment_name: value
                .attachment_name
                .as_ref()
                .map(|attachment_name| attachment_name.as_str()),
            attachment_mime: value
                .attachment_mime
                .as_ref()
                .map(|attachment_mime| attachment_mime.as_str()),
            hub_id: value.hub_id.get(),
        }
    }
}

impl<'a> From<&'a DomainUpdateEmailRecipient> for UpdateEmailRecipient<'a> {
    fn from(value: &'a DomainUpdateEmailRecipient) -> Self {
        Self {
            opened: value.opened,
            is_sent: value.is_sent,
            replied: value.replied,
            reply: value.reply.as_ref().map(|reply| reply.as_str()),
            updated_at: Some(chrono::Utc::now().naive_utc()),
        }
    }
}
