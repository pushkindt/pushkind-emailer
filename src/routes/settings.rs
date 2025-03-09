use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::forms::settings::{ActivateHubForm, AddHubForm, DeleteHubForm, SaveHubForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::repository::hub::{create_hub, delete_hub, get_hub, list_hubs, update_hub};
use crate::repository::user::set_user_hub;

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

    let flash_messages = get_flash_messages(&mut session);
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
            .render("settings/settings.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/settings/add")]
pub async fn settings_add(
    _: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddHubForm>,
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

#[post("/settings/activate")]
pub async fn settings_activate(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<ActivateHubForm>,
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

    match set_user_hub(&mut conn, user.0.id, Some(form.hub_id)) {
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
    web::Form(form): web::Form<SaveHubForm>,
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

    match update_hub(&mut conn, &form.into()) {
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

#[post("/settings/delete")]
pub async fn settings_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteHubForm>,
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

    match delete_hub(&mut conn, user.0.id, form.id) {
        Ok(_) => {
            add_flash_message(&mut session, "success", "Хаб удалён.");
        }
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при удалении хаба: {}", err),
            );
        }
    };
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/settings"))
        .finish()
}
