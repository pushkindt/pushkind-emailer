use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::models::{Hub, add_flash_message, get_flash_messages};
use crate::repository::{create_hub, get_hub, list_hubs, set_user_hub, update_hub};
use crate::routes::auth::AuthenticatedUser;

#[get("/")]
pub async fn index(user: AuthenticatedUser, session: Session) -> impl Responder {
    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/index.html", &context)
            .unwrap_or_default(),
    )
}

#[get("/recipients")]
pub async fn recipients(user: AuthenticatedUser, session: Session) -> impl Responder {
    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");
    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/recipients.html", &context)
            .unwrap_or_default(),
    )
}

#[get("/settings")]
pub async fn settings(
    user: AuthenticatedUser,
    mut session: Session,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    let hubs = match list_hubs(&mut conn) {
        Ok(hubs) => hubs,
        Err(err) => {
            error!("Error fetching hubs: {}", err);
            vec![]
        }
    };

    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("hubs", &hubs);
    context.insert("current_user", &user);
    context.insert("current_page", "settings");

    if user.0.hub_id.is_some() {
        if let Ok(hub) = get_hub(&mut conn, user.0.hub_id.unwrap()) {
            context.insert("current_hub", &hub);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/settings.html", &context)
            .unwrap_or_default(),
    )
}

#[derive(Deserialize)]
struct AddHubRequest {
    hub_name: String,
}

#[post("/settings/add")]
pub async fn settings_add(
    _: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddHubRequest>,
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

    match create_hub(&mut conn, &form.hub_name) {
        Ok(_) => {
            add_flash_message(&mut session, "success", "Хаб успешно создан.");
        }
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при создании хаба: {}", err),
            );
        }
    };
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/settings"))
        .finish()
}

#[derive(Deserialize)]
struct ActivateHubRequest {
    hub_id: i32,
}

#[post("/settings/activate")]
pub async fn settings_activate(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<ActivateHubRequest>,
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

    match set_user_hub(&mut conn, user.0.id, form.hub_id) {
        Ok(_) => {
            add_flash_message(&mut session, "success", "Хаб выбран.");
        }
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при выборе хаба: {}", err),
            );
        }
    };
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/settings"))
        .finish()
}

#[post("/settings/save")]
pub async fn settings_save(
    _: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<Hub>,
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

    match update_hub(&mut conn, &form) {
        Ok(_) => {
            add_flash_message(&mut session, "success", "Хаб сохранён.");
        }
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при изменении хаба: {}", err),
            );
        }
    };
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/settings"))
        .finish()
}
