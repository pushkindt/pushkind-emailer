use bcrypt::{DEFAULT_COST, hash, verify};
use diesel::prelude::*;
use log::error;

use crate::models::user::{NewUserDb, UserDb};

pub fn create_user(
    conn: &mut SqliteConnection,
    email: &str,
    password: &str,
) -> QueryResult<UserDb> {
    use crate::schema::users;

    let hashed_password = hash(password, DEFAULT_COST).map_err(|err| {
        error!("Error hashing password: {}", err);
        diesel::result::Error::RollbackTransaction
    })?;

    let new_user = NewUserDb {
        email: email,
        password: &hashed_password,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
}

pub fn find_user_by_email(conn: &mut SqliteConnection, email: &str) -> QueryResult<UserDb> {
    use crate::schema::users;
    users::table.filter(users::email.eq(email)).first(conn)
}

pub fn verify_password(stored_hash: &str, password: &str) -> bool {
    verify(password, stored_hash).unwrap_or(false)
}

pub fn set_user_hub(
    conn: &mut SqliteConnection,
    user_id: i32,
    hub_id: Option<i32>,
) -> QueryResult<usize> {
    use crate::schema::users;

    diesel::update(users::table.find(user_id))
        .set(users::hub_id.eq(hub_id))
        .execute(conn)
}

pub fn get_user(conn: &mut SqliteConnection, user_id: i32) -> QueryResult<UserDb> {
    use crate::schema::users;

    users::table.find(user_id).first(conn)
}
