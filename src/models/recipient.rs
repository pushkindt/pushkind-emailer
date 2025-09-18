use diesel::prelude::*;
use pushkind_common::models::emailer::hub::Hub;
use serde::Serialize;

use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Unsubscribe as DomainUnsubscribe,
};

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations, QueryableByName)]
#[diesel(table_name = pushkind_common::schema::emailer::recipients)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(foreign_derive)]
pub struct Recipient {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub hub_id: i32,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub fields: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = pushkind_common::schema::emailer::recipients)]
pub struct NewRecipient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub hub_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable, Serialize)]
#[diesel(table_name = pushkind_common::schema::emailer::recipient_fields)]
#[diesel(belongs_to(Recipient, foreign_key = recipient_id))]
#[diesel(primary_key(recipient_id, field))]
pub struct RecipientField {
    pub recipient_id: i32,
    pub field: String,
    pub value: String,
}

#[derive(QueryableByName)]
pub struct RecipientCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub count: i64,
}

#[derive(Queryable, Selectable, Serialize, QueryableByName)]
#[diesel(table_name = pushkind_common::schema::emailer::unsubscribes)]
#[diesel(primary_key(email, hub_id))]
pub struct Unsubscribe {
    pub email: String,
    pub hub_id: i32,
    pub reason: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl<'a> From<&'a DomainNewRecipient> for NewRecipient<'a> {
    fn from(value: &'a DomainNewRecipient) -> Self {
        Self {
            name: &value.name,
            email: &value.email,
            hub_id: value.hub_id,
        }
    }
}

impl From<Unsubscribe> for DomainUnsubscribe {
    fn from(value: Unsubscribe) -> Self {
        Self {
            email: value.email,
            hub_id: value.hub_id,
            reason: value.reason,
            unsubscribed_at: value.created_at,
        }
    }
}
