use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize, AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::hubs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Hub {
    pub id: i32,
    pub name: String,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub server: Option<String>,
    pub port: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::hubs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewHub<'a> {
    pub name: &'a str,
}
