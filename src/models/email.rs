use diesel::prelude::*;
use serde::Serialize;

use crate::models::user::User;

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = crate::schema::emails)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Email {
    pub id: i32,
    pub user_id: i32,
    pub message: String,
    pub created_at: chrono::NaiveDateTime,
    pub is_sent: bool,
    pub subject: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::emails)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewEmail<'a> {
    pub user_id: i32,
    pub message: &'a str,
    pub created_at: &'a chrono::NaiveDateTime,
    pub is_sent: bool,
    pub subject: Option<&'a str>,
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
