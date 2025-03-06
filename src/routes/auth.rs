use actix_identity::Identity;
use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpMessage, Responder, get, post, web};
use actix_web::{HttpRequest, HttpResponse};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::forms::auth::{LoginForm, RegisterForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::repository::user::{create_user, find_user_by_email, verify_password};

#[post("/login")]
pub async fn login(
    request: HttpRequest,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<LoginForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    match find_user_by_email(&mut conn, &form.email) {
        Ok(user) => {
            if verify_password(&user.password, &form.password) {
                Identity::login(&request.extensions(), user.email).unwrap();

                HttpResponse::SeeOther()
                    .insert_header((header::LOCATION, "/"))
                    .finish()
            } else {
                add_flash_message(&mut session, "danger", "Не верный пароль.");
                HttpResponse::SeeOther()
                    .insert_header((header::LOCATION, "/auth/signin"))
                    .finish()
            }
        }
        Err(_) => {
            add_flash_message(&mut session, "danger", "Пользователь не существует.");
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/auth/signin"))
                .finish()
        }
    }
}

#[post("/register")]
pub async fn register(
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<RegisterForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    match create_user(&mut conn, &form.email, &form.password) {
        Ok(_) => {
            add_flash_message(&mut session, "success", "Пользователь может войти.");
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/auth/signin"))
                .finish()
        }
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при создании пользователя: {}", err),
            );

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
        let flash_messages = get_flash_messages(&session);

        let mut context = Context::new();

        context.insert("alerts", &flash_messages);

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
        let flash_messages = get_flash_messages(&session);

        let mut context = Context::new();

        context.insert("alerts", &flash_messages);

        HttpResponse::Ok().body(
            TEMPLATES
                .render("auth/register.html", &context)
                .unwrap_or_default(),
        )
    }
}
