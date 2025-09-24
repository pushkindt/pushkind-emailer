use std::error::Error;
use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::email::{NewEmail, UpdateEmailRecipient};
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::models::emailer::zmq::ZMQSendEmailMessage;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;
use tera::Tera;

use crate::domain::recipient::CSVExportRecipient;
use crate::forms::main::{DeleteEmailForm, ResendEmailForm, SendEmailForm};
use crate::models::config::ServerConfig;
use crate::repository::{
    DieselRepository, EmailListQuery, EmailReader, EmailWriter, GroupListQuery, GroupReader,
    RecipientListQuery, RecipientReader,
};

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

    let retry = match params.retry {
        Some(email_id) => repo.get_email_by_id(email_id, user.hub_id).ok(),
        None => None,
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "index",
        &common_config.auth_service_url,
    );
    context.insert("retry", &retry);

    let query = RecipientListQuery::new(user.hub_id);

    let recipients = match repo.list_recipients(query) {
        Ok((_total, recipients)) => recipients,
        Err(e) => {
            log::error!("Failed to list recipients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let query = GroupListQuery::new(user.hub_id);
    let groups = match repo.list_groups(query) {
        Ok((_total, groups)) => groups,
        Err(e) => {
            log::error!("Failed to list groups: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let page = params.page.unwrap_or(1);

    let query = EmailListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let emails = match repo.list_emails(query) {
        Ok((total, emails)) => Paginated::new(emails, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE)),
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

    let new_email: NewEmail = match form.to_new_email(user.hub_id, repo.get_ref()) {
        Ok(new_email) => new_email,
        Err(err) => {
            return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {err}"));
        }
    };

    if new_email.recipients.is_empty() {
        return HttpResponse::Ok().body("Не указаны получатели.");
    }

    let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user, new_email)));

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

    let email = match repo.get_email_by_id(form.id, user.hub_id) {
        Ok(Some(email)) => email.email,
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

    let email = match repo.get_email_by_id(form.id, user.hub_id) {
        Ok(Some(email)) => email,
        _ => {
            FlashMessage::error("Сообщение не найдено.").send();
            return redirect("/");
        }
    };

    let zmq_message = ZMQSendEmailMessage::RetryEmail((email.email.id, user.hub_id));

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
                reply: None,
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
    let email = match repo.get_email_by_id(email_id, user.hub_id) {
        Ok(Some(email)) => email,
        Ok(_) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to get email: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut writer = csv::Writer::from_writer(vec![]);
    for recipient in email.recipients {
        let recipient = CSVExportRecipient::from(recipient);
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
