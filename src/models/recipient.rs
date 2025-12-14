//! Diesel models backing recipient persistence.
use crate::models::hub::Hub;
use diesel::prelude::*;
use serde::Serialize;

use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Unsubscribe as DomainUnsubscribe,
};
use crate::domain::types::{HubId, RecipientEmail, TypeConstraintError, UnsubscribeReason};

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations, QueryableByName)]
#[diesel(table_name = crate::schema::recipients)]
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
#[diesel(table_name = crate::schema::recipients)]
pub struct NewRecipient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub hub_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable, Serialize)]
#[diesel(table_name = crate::schema::recipient_fields)]
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
#[diesel(table_name = crate::schema::unsubscribes)]
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
            name: value.name.as_str(),
            email: value.email.as_str(),
            hub_id: value.hub_id.get(),
        }
    }
}

impl TryFrom<Unsubscribe> for DomainUnsubscribe {
    type Error = TypeConstraintError;

    fn try_from(value: Unsubscribe) -> Result<Self, Self::Error> {
        Ok(Self {
            email: RecipientEmail::new(value.email)?,
            hub_id: HubId::try_from(value.hub_id)?,
            reason: value.reason.map(UnsubscribeReason::try_from).transpose()?,
            unsubscribed_at: value.created_at,
        })
    }
}
