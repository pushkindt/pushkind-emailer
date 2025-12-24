//! Settings and history HTTP handlers.
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use tera::Tera;

use crate::dto::settings::ExportedHistory;
use crate::forms::settings::SaveHubForm;
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::settings::{
    export_history, load_history, load_settings_overview, load_unsubscribed, save_hub,
};

#[get("/settings")]
pub async fn settings_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let data = match load_settings_overview(&user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/");
        }
        Err(err) => {
            log::error!("Error getting hub: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "settings",
        &server_config.auth_service_url,
    );
    context.insert("current_hub", &data.hub);

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
    let data = match load_unsubscribed(&user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(err) => {
            log::error!("Error getting unsubscribed recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "unsubscribed",
        &server_config.auth_service_url,
    );
    context.insert("unsubscribed_list", &data.unsubscribed);

    render_template(&tera, "settings/unsubscribed.html", &context)
}

#[get("/history")]
pub async fn history_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let data = match load_history(&user, repo.get_ref(), &server_config) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(err) => {
            log::error!("Error getting history recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "history",
        &common_config.auth_service_url,
    );
    context.insert("history_list", &data.history);
    context.insert("crm_service_url", &data.crm_service_url);

    render_template(&tera, "settings/history.html", &context)
}

#[get("/history/download")]
pub async fn history_download(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match export_history(&user, repo.get_ref()) {
        Ok(ExportedHistory { bytes }) => HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                "attachment; filename=\"recipients_history.csv\"",
            ))
            .body(bytes),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Error exporting history: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/settings/save")]
pub async fn settings_save(
    web::Form(form): web::Form<SaveHubForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match save_hub(form, &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Хаб сохранён.").send();
            redirect("/settings")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Error updating hub: {err}");
            FlashMessage::error("Ошибка при изменении хаба.").send();
            redirect("/settings")
        }
    }
}
