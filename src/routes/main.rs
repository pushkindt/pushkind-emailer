use std::error::Error;

use actix_identity::Identity;
use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use log::error;
use serde::Deserialize;
use tera::Context;

use crate::db::{DbPool, get_db_connection};
use crate::forms::main::{DeleteEmailForm, SendEmailForm};
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::email::{
    create_email, get_email, get_email_recipient, get_email_recipients,
    get_hub_all_emails_with_recipients, remove_email, reset_email_sent_and_opened_status,
    set_email_recipient_opened_status, update_email_num_opened,
};
use crate::repository::recipient::{
    get_hub_all_groups, get_hub_all_recipients, get_hub_all_recipients_fields,
};
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};
use crate::utils::{read_attachment_file, send_zmq_email_id};

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
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let (retry, retry_recipients) = match params.retry {
        Some(email_id) => (
            get_email(&mut conn, email_id).ok(),
            get_email_recipients(&mut conn, email_id).ok(),
        ),
        None => (None, None),
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
    context.insert("retry_recipients", &retry_recipients);
    context.insert("home_url", &server_config.auth_service_url);

    if let Ok(recipients) = get_hub_all_recipients(&mut conn, user.hub_id) {
        context.insert("recipients", &recipients);
    }
    if let Ok(groups) = get_hub_all_groups(&mut conn, user.hub_id) {
        context.insert("groups", &groups);
    }
    if let Ok(emails) = get_hub_all_emails_with_recipients(&mut conn, user.hub_id) {
        context.insert("emails", &emails);
    }
    if let Ok(custom_fields) = get_hub_all_recipients_fields(&mut conn, user.hub_id) {
        context.insert("custom_fields", &custom_fields);
    }

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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let mut form = match form {
        Ok(form) => form,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {}", err)),
    };

    let (attchment_name, attachement_mime, attachment) =
        if let Some(attachment) = form.attachment.as_mut() {
            match read_attachment_file(attachment) {
                Ok((name, mime, data)) => (name, mime, data),
                Err(err) => {
                    error!("Ошибка при чтении файла: {}", err);
                    (None, None, None)
                }
            }
        } else {
            (None, None, None)
        };
    match create_email(
        &mut conn,
        form.subject.0.as_deref(),
        &form.message,
        &form.recipients,
        attachment.as_deref(),
        attchment_name.as_deref(),
        attachement_mime.as_deref(),
        user.hub_id,
    ) {
        Ok(email) => match send_zmq_email_id(email.id, &zmq_config) {
            Ok(_) => HttpResponse::Ok().body("Сообщение создано."),
            Err(err) => HttpResponse::Ok().body(format!(
                "Ошибка при добавлении сообщения в очередь: {}",
                err
            )),
        },
        Err(err) => HttpResponse::Ok().body(format!("Ошибка при создании сообщения: {}", err)),
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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match remove_email(&mut conn, form.id, user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Сообщение удалено.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при удалении сообщения: {}", err)).send();
        }
    }

    redirect("/")
}

#[post("/retry_email")]
pub async fn retry_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    zmq_config: web::Data<ServerConfig>,
    web::Form(form): web::Form<DeleteEmailForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match get_email(&mut conn, form.id) {
        Ok(email) if email.hub_id == user.hub_id => {
            match send_zmq_email_id(email.id, &zmq_config) {
                Ok(_) => match reset_email_sent_and_opened_status(&mut conn, email.id) {
                    Ok(_) => {
                        FlashMessage::success("Сообщение добавлено в очередь на отправку.").send();
                    }
                    Err(err) => {
                        FlashMessage::error(format!(
                            "Ошибка при добавлении сообщения в очередь: {}",
                            err
                        ))
                        .send();
                    }
                },
                Err(err) => {
                    FlashMessage::error(format!(
                        "Ошибка при добавлении сообщения в очередь: {}",
                        err
                    ))
                    .send();
                }
            }
        }
        Ok(_) => {
            FlashMessage::error("Вы не можете добавить это сообщение в очередь.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при поиске сообщения: {}", err)).send();
        }
    }

    redirect("/")
}

#[get("/track/{recipient_id}")]
pub async fn track_email(recipient_id: web::Path<i32>, pool: web::Data<DbPool>) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let recipient = match get_email_recipient(&mut conn, recipient_id.into_inner()) {
        Ok(recipient) => recipient,
        Err(err) => {
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if set_email_recipient_opened_status(&mut conn, recipient.id, true).is_err() {
        error!("Failed to update recipient status"); // Log the error for debugging
        return HttpResponse::InternalServerError().finish();
    }

    if update_email_num_opened(&mut conn, recipient.email_id).is_err() {
        error!("Failed to update email num_opened"); // Log the error for debugging
        return HttpResponse::InternalServerError().finish();
    }

    redirect("/assets/placeholder.png")
}

#[post("/logout")]
pub async fn logout(user: Identity) -> impl Responder {
    user.logout();
    redirect("/")
}

#[get("/na")]
pub async fn not_assigned(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
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
