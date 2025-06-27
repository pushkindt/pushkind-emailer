use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use tera::Context;

use crate::db::{DbPool, get_db_connection};
use crate::forms::settings::SaveHubForm;
use crate::models::auth::AuthenticatedUser;
use crate::models::hub::Hub;
use crate::repository::hub::{get_hub, update_hub};
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};

#[get("/settings")]
pub async fn settings(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "settings");

    let hub = match get_hub(&mut conn, user.hub_id) {
        Ok(hub) => hub,
        Err(_) => Hub::new(user.hub_id),
    };

    context.insert("current_hub", &hub);

    render_template("settings/settings.html", &context)
}

#[post("/settings/save")]
pub async fn settings_save(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<SaveHubForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match update_hub(&mut conn, &form.into()) {
        Ok(_) => {
            FlashMessage::success("Хаб сохранён.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при изменении хаба: {}", err)).send();
        }
    };
    redirect("/settings")
}
