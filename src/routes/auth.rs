use actix_identity::Identity;
use actix_session::Session;
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header;
use actix_web::{Error, FromRequest, HttpRequest, HttpResponse, dev::Payload};
use actix_web::{HttpMessage, Responder, get, post, web};

use serde::{Deserialize, Serialize};
use std::future::{Ready, ready};
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::models::Alert;
use crate::repository::{create_user, find_user_by_email, verify_password};

#[derive(Serialize)]
pub struct AuthenticatedUser(pub String);

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let identity = Identity::from_request(req, &mut Payload::None).into_inner();
        let pool = req.app_data::<web::Data<DbPool>>().cloned();

        if let (Ok(Some(uid)), Some(pool)) = (identity.map(|i| i.id().ok()), pool) {
            use crate::schema::users::dsl::*;
            use diesel::prelude::*;

            let mut conn = pool.get().expect("Couldn't get DB connection");
            let user_exists =
                diesel::dsl::select(diesel::dsl::exists(users.filter(email.eq(&uid))))
                    .get_result::<bool>(&mut conn)
                    .unwrap_or(false);
            if user_exists {
                return ready(Ok(AuthenticatedUser(uid)));
            }
        }
        ready(Err(ErrorUnauthorized("Unauthorized").into()))
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[post("/login")]
pub async fn login(
    request: HttpRequest,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<LoginRequest>,
    session: Session,
) -> impl Responder {
    let mut conn = pool.get().expect("Couldn't get DB connection");

    match find_user_by_email(&mut conn, &form.email) {
        Ok(user) => {
            if verify_password(&user.password, &form.password) {
                Identity::login(&request.extensions(), user.email).unwrap();

                HttpResponse::SeeOther()
                    .insert_header((header::LOCATION, "/"))
                    .finish()
            } else {
                session
                    .insert("flash_message", "Не верный пароль.")
                    .unwrap();
                HttpResponse::SeeOther()
                    .insert_header((header::LOCATION, "/auth/signin"))
                    .finish()
            }
        }
        Err(_) => {
            session
                .insert("flash_message", "Пользователь не существует.")
                .unwrap();
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/auth/signin"))
                .finish()
        }
    }
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
}

#[post("/register")]
pub async fn register(
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<RegisterRequest>,
    session: Session,
) -> impl Responder {
    let mut conn = pool.get().expect("Couldn't get DB connection");

    match create_user(&mut conn, &form.email, &form.password) {
        Ok(_) => {
            session
                .insert("flash_message", "Пользователь может войти.")
                .unwrap();
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/auth/signin"))
                .finish()
        }
        Err(err) => {
            session
                .insert(
                    "flash_message",
                    format!("Ошибка при создании пользователя: {}", err),
                )
                .unwrap();
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/auth/signup"))
                .finish()
        }
    }
}

#[post("/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/auth/signin"))
        .finish()
}

#[get("/signin")]
pub async fn signin(user: Option<Identity>, session: Session) -> impl Responder {
    if let Some(_) = user {
        HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/"))
            .finish()
    } else {
        let flash_message: Option<String> = session.get("flash_message").unwrap_or(None);
        session.remove("flash_message");

        let mut context = Context::new();

        if let Some(message) = flash_message {
            context.insert(
                "alerts",
                &vec![Alert {
                    category: "primary".to_string(),
                    message: message,
                }],
            );
        }

        HttpResponse::Ok().body(
            TEMPLATES
                .render("auth/login.html", &context)
                .unwrap_or_default(),
        )
    }
}

#[get("/signup")]
pub async fn signup(user: Option<Identity>, session: Session) -> impl Responder {
    if let Some(_) = user {
        HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/"))
            .finish()
    } else {
        let flash_message: Option<String> = session.get("flash_message").unwrap_or(None);
        session.remove("flash_message");

        let mut context = Context::new();

        if let Some(message) = flash_message {
            context.insert(
                "alerts",
                &vec![Alert {
                    category: "primary".to_string(),
                    message: message,
                }],
            );
        }
        HttpResponse::Ok().body(
            TEMPLATES
                .render("auth/register.html", &context)
                .unwrap_or_default(),
        )
    }
}
