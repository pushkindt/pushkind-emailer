use std::error::Error;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use serde::Deserialize;
use tera::Context;

use crate::domain::email::{NewEmail, UpdateEmailRecipient};
use crate::forms::main::{DeleteEmailForm, SendEmailForm};
use crate::models::config::ServerConfig;
use crate::repository::email::DieselEmailRepository;
use crate::repository::group::DieselGroupRepository;
use crate::repository::recipient::DieselRecipientRepository;
use crate::repository::{EmailReader, EmailWriter, GroupReader, RecipientReader};
use crate::routes::render_template;
use crate::utils::send_zmq_email_id;

#[derive(Deserialize)]
struct IndexQueryParams {
    retry: Option<i32>,
}

#[get("/")]
pub async fn index(
    params: web::Query<IndexQueryParams>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let recipient_repo = DieselRecipientRepository::new(&pool);
    let group_repo = DieselGroupRepository::new(&pool);
    let email_repo = DieselEmailRepository::new(&pool);

    let retry = match params.retry {
        Some(email_id) => email_repo.get_by_id(email_id).ok(),
        None => None,
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("retry", &retry);
    context.insert("home_url", &server_config.auth_service_url);

    let recipients = match recipient_repo.list(user.hub_id) {
        Ok(recipients) => recipients,
        Err(e) => {
            log::error!("Failed to list recipients: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let groups = match group_repo.list(user.hub_id) {
        Ok(groups) => groups,
        Err(e) => {
            log::error!("Failed to list groups: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let emails = match email_repo.list(user.hub_id) {
        Ok(emails) => emails,
        Err(e) => {
            log::error!("Failed to list emails: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let custom_fields = match recipient_repo.list_custom_fields(user.hub_id) {
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

    render_template("main/index.html", &context)
}

#[post("/send_email")]
pub async fn send_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    zmq_config: web::Data<ServerConfig>,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let email_repo = DieselEmailRepository::new(&pool);

    let form = match form {
        Ok(form) => form,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {err}")),
    };

    let mut new_email: NewEmail = form.0.into();
    new_email.hub_id = user.hub_id;

    match email_repo.create(&new_email) {
        Ok(email) => match send_zmq_email_id(email.email.id, &zmq_config) {
            Ok(_) => HttpResponse::Ok().body("Сообщение создано."),
            Err(err) => {
                HttpResponse::Ok().body(format!("Ошибка при добавлении сообщения в очередь: {err}"))
            }
        },
        Err(err) => HttpResponse::Ok().body(format!("Ошибка при создании сообщения: {err}")),
    }
}

#[post("/delete_email")]
pub async fn delete_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteEmailForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let email_repo = DieselEmailRepository::new(&pool);

    let email = match email_repo.get_by_id(form.id) {
        Ok(Some(email)) if email.email.hub_id == user.hub_id => email.email,
        _ => {
            FlashMessage::error("Сообщение не найдено.").send();
            return redirect("/");
        }
    };

    match email_repo.delete(email.id) {
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

#[get("/track/{recipient_id}")]
pub async fn track_email(recipient_id: web::Path<i32>, pool: web::Data<DbPool>) -> impl Responder {
    let email_repo = DieselEmailRepository::new(&pool);

    let recipient_id = recipient_id.into_inner();

    if email_repo
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

#[get("/na")]
pub async fn not_assigned(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    context.insert("home_url", &server_config.auth_service_url);

    render_template("main/not_assigned.html", &context)
}
