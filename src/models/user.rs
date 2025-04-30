use diesel::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i32,
    pub email: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub hub_id: Option<i32>,
}

// Conversion from database model to DTO
impl From<UserDb> for UserDto {
    fn from(user: UserDb) -> Self {
        UserDto {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
            hub_id: user.hub_id,
        }
    }
}
