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
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::recipients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewRecipient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub hub_id: i32,
}

#[derive(Queryable, Selectable, Serialize, Identifiable, Associations, Clone)]
#[diesel(table_name = crate::schema::groups)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub hub_id: i32,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewGroup<'a> {
    pub name: &'a str,
    pub hub_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable)]
#[diesel(table_name = crate::schema::groups_recipients)]
#[diesel(belongs_to(Recipient, foreign_key = recipient_id))]
#[diesel(belongs_to(Group, foreign_key = group_id))]
#[diesel(primary_key(group_id, recipient_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GroupRecipient {
    pub group_id: i32,
    pub recipient_id: i32,
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
