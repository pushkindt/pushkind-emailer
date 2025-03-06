use diesel::prelude::*;
use serde::Serialize;

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::recipients)]
#[diesel(belongs_to(Hub))]
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

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::groups)]
#[diesel(belongs_to(Hub))]
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

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::groups_recipients)]
#[diesel(belongs_to(Recipient))]
#[diesel(belongs_to(Group))]
#[diesel(primary_key(group_id, recipient_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GroupRecipient {
    pub group_id: i32,
    pub recipient_id: i32,
}
