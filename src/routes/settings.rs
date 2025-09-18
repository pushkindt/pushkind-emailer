use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::hub::NewHub;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use tera::Tera;

use crate::forms::settings::SaveHubForm;
use crate::repository::{DieselRepository, HubReader, HubWriter, RecipientReader};

#[get("/settings")]
pub async fn settings_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "settings",
        &server_config.auth_service_url,
    );

    let hub = match repo.get_hub_by_id(user.hub_id) {
        Ok(Some(hub)) => hub,
        Ok(None) => match repo.create_hub(&NewHub::new(user.hub_id)) {
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

#[get("/unsubscribed")]
pub async fn unsubscribed_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "unsubscribed",
        &server_config.auth_service_url,
    );

    let unsubscribed_list = match repo.list_unsubscribed_recipients(user.hub_id) {
        Ok(unsubscribed) => unsubscribed,
        Err(e) => {
            log::error!("Error getting unsubscribed recipients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("unsubscribed_list", &unsubscribed_list);

    render_template(&tera, "settings/unsubscribed.html", &context)
}

#[post("/settings/save")]
pub async fn settings_save(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<SaveHubForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "admin", None) {
        return response;
    };

    let hub = match repo.get_hub_by_id(user.hub_id) {
        Ok(Some(hub)) => hub,
        Ok(None) => match repo.create_hub(&NewHub::new(user.hub_id)) {
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

    match repo.update_hub(hub.id, &form.into()) {
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
