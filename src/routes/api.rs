//! Actix routes serving the Emailer API surface.

use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::services::errors::ServiceError;

use crate::dto::main::IndexQueryParams;
use crate::dto::recipients::RecipientsQueryParams;
use crate::models::config::AppConfig;
use crate::repository::DieselRepository;
use crate::services::api as api_service;

#[get("/v1/iam")]
/// Return typed shell data for React-owned Emailer pages.
pub async fn api_v1_iam(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    match api_service::get_shell_data(&user, common_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer shell data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/emails")]
/// Return typed email collection data for the React-owned Emailer index.
pub async fn api_v1_emails(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_emails_data(
        params.into_inner(),
        &user,
        repo.get_ref(),
        app_config.get_ref(),
    ) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer emails data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/recipients")]
/// Return typed recipient collection data.
pub async fn api_v1_recipients(
    params: web::Query<RecipientsQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_recipients_data(
        params.into_inner(),
        &user,
        repo.get_ref(),
        app_config.get_ref(),
    ) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer recipients page data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/recipients/{recipient_id}")]
/// Return typed modal data for a single recipient.
pub async fn api_v1_recipient(
    recipient_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_recipient_modal_data(recipient_id.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer recipient modal data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/groups")]
/// Return typed group collection data.
pub async fn api_v1_groups(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_groups_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer groups page data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/groups/{group_id}")]
/// Return typed modal data for a single group.
pub async fn api_v1_group(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_group_modal_data(group_id.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer group modal data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/hub-settings")]
/// Return typed hub settings data.
pub async fn api_v1_hub_settings(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_hub_settings_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer hub settings data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/unsubscribed-recipients")]
/// Return typed unsubscribed recipient collection data.
pub async fn api_v1_unsubscribed_recipients(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_unsubscribed_recipients_data(&user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer unsubscribed recipients data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/email-history")]
/// Return typed email history collection data.
pub async fn api_v1_email_history(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_email_history_data(&user, repo.get_ref(), app_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load Emailer history data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/no-access")]
/// Return typed page data for the React-owned Emailer no-access page.
pub async fn api_v1_no_access(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    HttpResponse::Ok().json(api_service::get_no_access_data(
        &user,
        common_config.get_ref(),
    ))
}
