use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use tera::Tera;

use crate::domain::hub::NewHub;
use crate::forms::settings::SaveHubForm;
use crate::repository::hub::DieselHubRepository;
use crate::repository::{HubReader, HubWriter};

#[get("/settings")]
pub async fn settings(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let hub_repo = DieselHubRepository::new(&pool);

    let mut context = base_context(
        &flash_messages,
        &user,
        "settings",
        &server_config.auth_service_url,
    );

    let hub = match hub_repo.get_by_id(user.hub_id) {
        Ok(Some(hub)) => hub,
        Ok(None) => match hub_repo.create(&NewHub::new(user.hub_id)) {
            Ok(hub) => hub,
            Err(e) => {
                log::error!("Error creating hub: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        },
        Err(e) => {
            log::error!("Error getting hub: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("current_hub", &hub);

    render_template(&tera, "settings/settings.html", &context)
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
                log::error!("Error creating hub: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        },
        Err(e) => {
            log::error!("Error getting hub: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match hub_repo.update(hub.id, &(&form).into()) {
        Ok(_) => {
            FlashMessage::success("Хаб сохранён.").send();
        }
        Err(err) => {
            log::error!("Error updating hub: {err}");
            FlashMessage::error("Ошибка при изменении хаба.").send();
        }
    };
    redirect("/settings")
}
