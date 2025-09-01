use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use pushkind_common::domain::email as domain;
use serde::Serialize;

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
    pub name: Option<String>,
    pub reply: Option<String>,
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
    pub name: Option<&'a str>,
}

impl Email {
    pub fn get_recipients(
        &self,
        conn: &mut SqliteConnection,
    ) -> Result<Vec<EmailRecipient>, diesel::result::Error> {
        use crate::schema::email_recipients::dsl::*;
        email_recipients
            .filter(email_id.eq(self.id))
            .load::<EmailRecipient>(conn)
    }
}

impl From<Email> for domain::Email {
    fn from(value: Email) -> Self {
        Self {
            id: value.id,
            message: value.message,
            created_at: value.created_at,
            is_sent: value.is_sent,
            subject: value.subject,
            attachment: value.attachment,
            attachment_name: value.attachment_name,
            attachment_mime: value.attachment_mime,
            num_sent: value.num_sent,
            num_opened: value.num_opened,
            num_replied: value.num_replied,
            hub_id: value.hub_id,
        }
    }
}

impl From<EmailRecipient> for domain::EmailRecipient {
    fn from(value: EmailRecipient) -> Self {
        Self {
            id: value.id,
            email_id: value.email_id,
            address: value.address,
            opened: value.opened,
            updated_at: value.updated_at,
            is_sent: value.is_sent,
            replied: value.replied,
            name: value.name,
            reply: value.reply,
        }
    }
}

impl<'a> From<&'a domain::NewEmail> for NewEmail<'a> {
    fn from(value: &'a domain::NewEmail) -> Self {
        Self {
            message: &value.message,
            created_at: Utc::now().naive_utc(),
            is_sent: false,
            subject: value.subject.as_deref(),
            attachment: value.attachment.as_deref(),
            attachment_name: value.attachment_name.as_deref(),
            attachment_mime: value.attachment_mime.as_deref(),
            hub_id: value.hub_id,
        }
    }
}
