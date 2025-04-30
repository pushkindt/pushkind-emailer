use std::future::{Ready, ready};

use actix_identity::Identity;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::web;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload};
use diesel::prelude::*;
use serde::Serialize;

use crate::db::DbPool;
use crate::models::user::UserDb;

#[derive(Serialize)]
pub struct AuthenticatedUser(pub UserDb);

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let identity = Identity::from_request(req, &mut Payload::None)
            .into_inner()
            .map(|i| i.id().ok());
        let pool = req.app_data::<web::Data<DbPool>>();

        if let (Ok(Some(uid)), Some(pool)) = (identity, pool) {
            use crate::schema::users;

            let mut conn = match pool.get() {
                Ok(conn) => conn,
                Err(_) => {
                    return ready(Err(ErrorInternalServerError("DB connection error")));
                }
            };

            match users::table
                .filter(users::email.eq(&uid))
                .first::<UserDb>(&mut conn)
            {
                Ok(user) => return ready(Ok(AuthenticatedUser(user))),
                Err(_) => return ready(Err(ErrorUnauthorized("Invalid user"))),
            }
        }
        ready(Err(ErrorUnauthorized("Unauthorized")))
    }
}
