//! Settings and history HTTP handlers.
use actix_files::NamedFile;
use actix_web::{Either, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{check_role, redirect};
use pushkind_common::services::errors::ServiceError;

use crate::SERVICE_ACCESS_ROLE;
use crate::SERVICE_ADMIN_ROLE;
use crate::dto::api::{ApiMutationErrorDto, ApiMutationSuccessDto};
use crate::dto::settings::ExportedHistory;
use crate::forms::settings::{SaveHubForm, SaveHubPayload};
use crate::frontend::open_frontend_html;
use crate::repository::DieselRepository;
use crate::services::settings::{export_history, save_hub};

#[get("/settings")]
pub async fn settings_show(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ADMIN_ROLE, &user.roles) {
        return Either::Right(redirect("/"));
    }

    match open_frontend_html("assets/dist/app/settings.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer settings document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

#[get("/unsubscribed")]
pub async fn unsubscribed_show(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Either::Right(redirect("/na"));
    }

    match open_frontend_html("assets/dist/app/unsubscribed.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer unsubscribed document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

#[get("/history")]
pub async fn history_show(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Either::Right(redirect("/na"));
    }

    match open_frontend_html("assets/dist/app/history.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer history document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
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
        Err(ServiceError::Unauthorized) => redirect("/na"),
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
    let payload: SaveHubPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    match save_hub(payload, &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Хаб сохранён.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Error updating hub: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при изменении хаба.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}
