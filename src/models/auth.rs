use actix_identity::Identity;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::web;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload};
use serde::Serialize;
use std::future::{Ready, ready};

use crate::db::DbPool;
use crate::models::user::User;

#[derive(Serialize)]
pub struct AuthenticatedUser(pub User);

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let identity = Identity::from_request(req, &mut Payload::None).into_inner();
        let pool = req.app_data::<web::Data<DbPool>>().cloned();

        if let (Ok(Some(uid)), Some(pool)) = (identity.map(|i| i.id().ok()), pool) {
            use crate::schema::users::dsl::*;
            use diesel::prelude::*;

            let mut conn = match pool.get() {
                Ok(conn) => conn,
                Err(_) => {
                    return ready(Err(ErrorInternalServerError("DB connection error")));
                }
            };

            match users.filter(email.eq(&uid)).first::<User>(&mut conn) {
                Ok(user) => return ready(Ok(AuthenticatedUser(user))),
                Err(_) => return ready(Err(ErrorUnauthorized("Invalid user"))),
            }
        }
        ready(Err(ErrorUnauthorized("Unauthorized")))
    }
}
