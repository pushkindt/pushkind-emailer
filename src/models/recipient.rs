use diesel::prelude::*;
use serde::Serialize;

use crate::models::hub::Hub;

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(table_name = crate::schema::recipients)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Recipient {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub hub_id: i32,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub unsubscribed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::recipients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewRecipient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub hub_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable, Serialize)]
#[diesel(table_name = crate::schema::recipient_fields)]
#[diesel(belongs_to(Recipient, foreign_key = recipient_id))]
#[diesel(primary_key(recipient_id, field))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct RecipientField {
    pub recipient_id: i32,
    pub field: String,
    pub value: String,
}

use crate::domain::recipient as domain;

impl From<Recipient> for domain::Recipient {
    fn from(value: Recipient) -> Self {
        Self {
            id: value.id,
            name: value.name,
            email: value.email,
            hub_id: value.hub_id,
            fields: std::collections::HashMap::new(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            unsubscribed_at: value.unsubscribed_at,
        }
    }
}
