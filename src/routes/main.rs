//! Main email workflow HTTP handlers.
use std::error::Error;
use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use pushkind_common::zmq::ZmqSender;
use tera::Tera;

use crate::dto::main::{ExportedEmailRecipients, IndexQueryParams};
use crate::forms::main::SendEmailForm;
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::main::{
    delete_email as delete_email_service,
    export_email_recipients as export_email_recipients_service, load_index_page, mark_email_opened,
    queue_email_retry, queue_new_email,
};

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
    let data = match load_index_page(params.into_inner(), &user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
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

#[post("/email/send")]
pub async fn send_email(
    user: AuthenticatedUser,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form = match form {
        Ok(form) => form.0,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {err}")),
    };

    match queue_new_email(form, &user, repo.get_ref(), zmq_sender.as_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Сообщение добавлено в очередь."),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => HttpResponse::Ok().body(message),
        Err(err) => {
            log::error!("Failed to queue email: {err}");
            HttpResponse::InternalServerError().body("Ошибка при добавлении сообщения в очередь.")
        }
    }
}

#[post("/email/{email_id}/delete")]
pub async fn delete_email(
    email_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_email_service(email_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Сообщение удалено.").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Сообщение не найдено.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to delete email: {err}");
            FlashMessage::error("Ошибка при удалении сообщения.").send();
            redirect("/")
        }
    }
}

#[post("/email/{email_id}/resend")]
pub async fn resend_email(
    email_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match queue_email_retry(
        email_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.as_ref(),
    )
    .await
    {
        Ok(_) => {
            FlashMessage::success("Сообщение добавлено в очередь.").send();
            redirect("/")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Сообщение не найдено.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
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

    match mark_email_opened(recipient_id, repo.get_ref()) {
        Ok(_) => redirect("/assets/placeholder.png"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to update recipient status for {recipient_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/email/{email_id}/recipients/export")]
pub async fn export_email_recipients(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    email_id: web::Path<i32>,
) -> impl Responder {
    let email_id = email_id.into_inner();

    match export_email_recipients_service(email_id, &user, repo.get_ref()) {
        Ok(ExportedEmailRecipients { filename, bytes }) => HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(bytes),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to export recipients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
