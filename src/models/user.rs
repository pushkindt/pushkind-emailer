use diesel::prelude::*;
use serde::Serialize;

use crate::models::hub::Hub;

#[derive(Queryable, Selectable, Serialize, Associations, Identifiable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserDb {
    pub id: i32,
    pub email: String,
    pub password: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub hub_id: Option<i32>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewUserDb<'a> {
    pub email: &'a str,
    pub password: &'a str,
}
