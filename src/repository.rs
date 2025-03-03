use crate::models::{NewUser, User};
use bcrypt::{DEFAULT_COST, hash, verify};
use diesel::prelude::*;

pub fn create_user(conn: &mut SqliteConnection, email: &str, password: &str) -> QueryResult<User> {
    use crate::schema::users::dsl::{id, users};

    let password = hash(password, DEFAULT_COST).expect("Error hashing password");

    let new_user = NewUser {
        email,
        password: &password,
    };

    diesel::insert_into(users).values(&new_user).execute(conn)?;

    users.order(id.desc()).first(conn)
}

pub fn find_user_by_email(conn: &mut SqliteConnection, uname: &str) -> QueryResult<User> {
    use crate::schema::users::dsl::*;
    users.filter(email.eq(uname)).first(conn)
}

pub fn verify_password(stored_hash: &str, password: &str) -> bool {
    verify(password, stored_hash).unwrap_or(false)
}
