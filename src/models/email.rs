use diesel::prelude::*;
use serde::Serialize;

use crate::models::hub::Hub;

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(table_name = crate::schema::emails)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Email {
    pub id: i32,
    pub message: String,
    pub created_at: chrono::NaiveDateTime,
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
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewEmail<'a> {
    pub message: &'a str,
    pub created_at: &'a chrono::NaiveDateTime,
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
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EmailRecipient {
    pub id: i32,
    pub email_id: i32,
    pub address: String,
    pub opened: bool,
    pub updated_at: chrono::NaiveDateTime,
    pub is_sent: bool,
    pub replied: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::email_recipients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewEmailRecipient<'a> {
    pub email_id: i32,
    pub address: &'a str,
    pub opened: bool,
    pub updated_at: &'a chrono::NaiveDateTime,
    pub is_sent: bool,
    pub replied: bool,
}

use crate::domain::email as domain;

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
        }
    }
}
