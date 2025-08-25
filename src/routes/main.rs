use std::error::Error;
use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::email::{NewEmail, UpdateEmailRecipient};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::models::zmq::emailer::ZMQSendEmailMessage;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use tera::Tera;

use crate::forms::main::{DeleteEmailForm, ResendEmailForm, SendEmailForm};
use crate::repository::{DieselRepository, EmailReader, EmailWriter, GroupReader, RecipientReader};

#[derive(Deserialize)]
struct IndexQueryParams {
    retry: Option<i32>,
}

#[get("/")]
pub async fn index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let retry = match params.retry {
        Some(email_id) => repo.get_email_by_id(email_id).ok(),
        None => None,
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &server_config.auth_service_url,
    );
    context.insert("retry", &retry);

    let recipients = match repo.list_recipients(user.hub_id) {
        Ok(recipients) => recipients,
        Err(e) => {
            log::error!("Failed to list recipients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let groups = match repo.list_groups(user.hub_id) {
        Ok(groups) => groups,
        Err(e) => {
            log::error!("Failed to list groups: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let emails = match repo.list_emails(user.hub_id) {
        Ok(emails) => emails,
        Err(e) => {
            log::error!("Failed to list emails: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let custom_fields = match repo.list_custom_fields(user.hub_id) {
        Ok(custom_fields) => custom_fields,
        Err(e) => {
            log::error!("Failed to list custom fields: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("recipients", &recipients);
    context.insert("groups", &groups);
    context.insert("emails", &emails);
    context.insert("custom_fields", &custom_fields);

    render_template(&tera, "main/index.html", &context)
}

#[post("/send_email")]
pub async fn send_email(
    user: AuthenticatedUser,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let form = match form {
        Ok(form) => form,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {err}")),
    };

    let mut new_email: NewEmail = form.0.into();
    new_email.hub_id = user.hub_id;

    let zmq_message = ZMQSendEmailMessage::NewEmail(new_email);

    match zmq_sender.send_json(&zmq_message).await {
        Ok(_) => HttpResponse::Ok().body("Сообщение добавлено в очередь."),
        Err(err) => {
            HttpResponse::Ok().body(format!("Ошибка при добавлении сообщения в очередь: {err}"))
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

    let email = match repo.get_email_by_id(form.id) {
        Ok(Some(email)) if email.email.hub_id == user.hub_id => email.email,
        _ => {
            FlashMessage::error("Сообщение не найдено.").send();
            return redirect("/");
        }
    };

    match repo.delete_email(email.id) {
        Ok(_) => {
            FlashMessage::success("Сообщение удалено.").send();
        }
        Err(err) => {
            log::error!("Failed to delete email: {err}");
            FlashMessage::error("Ошибка при удалении сообщения.").send();
        }
    }

    redirect("/")
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

    let email = match repo.get_email_by_id(form.id) {
        Ok(Some(email)) if email.email.hub_id == user.hub_id => email,
        _ => {
            FlashMessage::error("Сообщение не найдено.").send();
            return redirect("/");
        }
    };

    let zmq_message = ZMQSendEmailMessage::RetryEmail(email.email.id);

    match zmq_sender.send_json(&zmq_message).await {
        Ok(_) => HttpResponse::Ok().body("Сообщение добавлено в очередь повторно."),
        Err(err) => {
            HttpResponse::Ok().body(format!("Ошибка при добавлении сообщения в очередь: {err}"))
        }
    };

    redirect("/")
}

#[get("/track/{recipient_id}")]
pub async fn track_email(
    recipient_id: web::Path<i32>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let recipient_id = recipient_id.into_inner();

    if repo
        .update_recipient(
            recipient_id,
            &UpdateEmailRecipient {
                opened: Some(true),
                is_sent: Some(true),
                replied: None,
            },
        )
        .is_err()
    {
        log::error!("Failed to update recipient status for {recipient_id}"); // Log the error for debugging
        return HttpResponse::InternalServerError().finish();
    }

    redirect("/assets/placeholder.png")
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

    let email_id = email_id.into_inner();
    let email = match repo.get_email_by_id(email_id) {
        Ok(Some(email)) if email.email.hub_id == user.hub_id => email,
        Ok(_) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to get email: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut writer = csv::Writer::from_writer(vec![]);
    for recipient in email.recipients {
        if let Err(err) = writer.serialize(recipient) {
            log::error!("Failed to write recipient to csv: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    let data = match writer.into_inner() {
        Ok(data) => data,
        Err(err) => {
            log::error!("Failed to finalize csv: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok()
        .content_type("text/csv")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"recipients_{email_id}.csv\""),
        ))
        .body(data)
}
