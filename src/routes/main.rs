use std::error::Error;
use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, ensure_role, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use tera::Tera;

use crate::forms::main::{DeleteEmailForm, ResendEmailForm, SendEmailForm};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::main::{ExportedEmailRecipients, MainService};

#[derive(Deserialize)]
struct IndexQueryParams {
    retry: Option<i32>,
    page: Option<usize>,
}

#[get("/")]
pub async fn index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let page = params.page.unwrap_or(1);
    let service = MainService::new(repo.get_ref());
    let data = match service.load_index_page(user.hub_id, params.retry, page) {
        Ok(data) => data,
        Err(err) => {
            log::error!("Failed to load index page: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &common_config.auth_service_url,
    );
    context.insert("retry", &data.retry_email);
    context.insert("recipients", &data.recipients);
    context.insert("groups", &data.groups);
    context.insert("emails", &data.emails);
    context.insert("custom_fields", &data.custom_fields);
    context.insert("crm_service_url", &server_config.crm_service_url);

    render_template(&tera, "main/index.html", &context)
}

#[post("/send_email")]
pub async fn send_email(
    user: AuthenticatedUser,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    repo: web::Data<DieselRepository>,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let form = match form {
        Ok(form) => form.0,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {err}")),
    };

    let service = MainService::new(repo.get_ref());
    match service
        .queue_new_email(user, form, zmq_sender.as_ref().as_ref())
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Сообщение добавлено в очередь."),
        Err(ServiceError::Form(message)) => HttpResponse::Ok().body(message),
        Err(err) => {
            log::error!("Failed to queue email: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/delete_email")]
pub async fn delete_email(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<DeleteEmailForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = MainService::new(repo.get_ref());
    match service.delete_email(user.hub_id, form) {
        Ok(_) => {
            FlashMessage::success("Сообщение удалено.").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Сообщение не найдено.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to delete email: {err}");
            FlashMessage::error("Ошибка при удалении сообщения.").send();
            redirect("/")
        }
    }
}

#[post("/resend_email")]
pub async fn resend_email(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    web::Form(form): web::Form<ResendEmailForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = MainService::new(repo.get_ref());
    match service
        .queue_email_retry(user.hub_id, form, zmq_sender.as_ref().as_ref())
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Сообщение добавлено в очередь повторно."),
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Сообщение не найдено.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to queue retry: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/track/{recipient_id}")]
pub async fn track_email(
    recipient_id: web::Path<i32>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let recipient_id = recipient_id.into_inner();
    let service = MainService::new(repo.get_ref());

    match service.mark_email_opened(recipient_id) {
        Ok(_) => redirect("/assets/placeholder.png"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to update recipient status for {recipient_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/emails/{email_id}/recipients/export")]
pub async fn export_email_recipients(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    email_id: web::Path<i32>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = MainService::new(repo.get_ref());
    let email_id = email_id.into_inner();

    match service.export_email_recipients(user.hub_id, email_id) {
        Ok(ExportedEmailRecipients { filename, bytes }) => HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(bytes),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to export recipients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
