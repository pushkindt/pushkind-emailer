use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use tera::Context;

use crate::domain::hub::{Hub, NewHub};
use crate::forms::settings::SaveHubForm;
use crate::repository::hub::DieselHubRepository;
use crate::repository::{HubReader, HubWriter};
use crate::routes::render_template;

#[get("/settings")]
pub async fn settings(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let hub_repo = DieselHubRepository::new(&pool);

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "settings");

    let hub = match hub_repo.get_by_id(user.hub_id) {
        Ok(Some(hub)) => hub,
        Ok(None) => match hub_repo.create(&NewHub::new(user.hub_id)) {
            Ok(hub) => hub,
            Err(e) => {
                log::error!("Error creating hub: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        },
        Err(e) => {
            log::error!("Error getting hub: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("current_hub", &hub);
    context.insert("home_url", &server_config.auth_service_url);

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

    let hub_repo = DieselHubRepository::new(&pool);

    let hub = match hub_repo.get_by_id(user.hub_id) {
        Ok(Some(hub)) => hub,
        Ok(None) => match hub_repo.create(&NewHub::new(user.hub_id)) {
            Ok(hub) => hub,
            Err(e) => {
                log::error!("Error creating hub: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        },
        Err(e) => {
            log::error!("Error getting hub: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    match hub_repo.update(hub.id, &(&form).into()) {
        Ok(_) => {
            FlashMessage::success("Хаб сохранён.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при изменении хаба: {}", err)).send();
        }
    };
    redirect("/settings")
}
