use std::error::Error;

use actix_multipart::form::MultipartForm;
use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::{DbPool, get_db_connection};
use crate::forms::main::{DeleteEmailForm, SendEmailForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::email::{
    create_email, get_email, get_user_all_emails_with_recipients, remove_email,
    reset_email_sent_and_opened_status, set_email_recipient_opened_status,
};
use crate::repository::recipient::{
    get_hub_all_groups, get_hub_all_recipients, get_hub_all_recipients_fields,
};
use crate::utils::send_zmq_email_id;

#[get("/")]
pub async fn index(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let flash_messages = get_flash_messages(&mut session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "index");

    if let Some(hub_id) = user.0.hub_id {
        if let Ok(recipients) = get_hub_all_recipients(&mut conn, hub_id) {
            context.insert("recipients", &recipients);
        }
        if let Ok(groups) = get_hub_all_groups(&mut conn, hub_id) {
            context.insert("groups", &groups);
        }
        if let Ok(emails) = get_user_all_emails_with_recipients(&mut conn, user.0.id) {
            context.insert("emails", &emails);
        }
        if let Ok(custom_fields) = get_hub_all_recipients_fields(&mut conn, hub_id) {
            context.insert("custom_fields", &custom_fields);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/index.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/send_email")]
pub async fn send_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    zmq_config: web::Data<ServerConfig>,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let form = match form {
        Ok(form) => form,
        Err(err) => return HttpResponse::Ok().body(format!("Ошибка при обработке формы: {}", err)),
    };

    if user.0.hub_id.is_some() {
        match create_email(&mut conn, form.0, user.0.id) {
            Ok(email) => match send_zmq_email_id(email.id, &zmq_config) {
                Ok(_) => {
                    return HttpResponse::Ok().body("Сообщение создано.");
                }
                Err(err) => {
                    return HttpResponse::Ok().body(format!(
                        "Ошибка при добавлении сообщения в очередь: {}",
                        err
                    ));
                }
            },
            Err(err) => {
                return HttpResponse::Ok().body(format!("Ошибка при создании сообщения: {}", err));
            }
        }
    } else {
        return HttpResponse::Ok().body("Вы не можете отправлять сообщения.");
    }
}

#[post("/delete_email")]
pub async fn delete_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteEmailForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
        match remove_email(&mut conn, form.id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Сообщение удалено.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении сообщения: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете удалять сообщения.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

#[post("/retry_email")]
pub async fn retry_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    zmq_config: web::Data<ServerConfig>,
    web::Form(form): web::Form<DeleteEmailForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
        match get_email(&mut conn, form.id) {
            Ok(email) if email.user_id == user.0.id => {
                match send_zmq_email_id(email.id, &zmq_config) {
                    Ok(_) => match reset_email_sent_and_opened_status(&mut conn, email.id) {
                        Ok(_) => {
                            add_flash_message(
                                &mut session,
                                "success",
                                "Сообщение добавлено в очередь на отправку.",
                            );
                        }
                        Err(err) => {
                            add_flash_message(
                                &mut session,
                                "danger",
                                &format!("Ошибка при добавлении сообщения в очередь: {}", err),
                            );
                        }
                    },
                    Err(err) => {
                        add_flash_message(
                            &mut session,
                            "danger",
                            &format!("Ошибка при добавлении сообщения в очередь: {}", err),
                        );
                    }
                }
            }
            Ok(_) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    "Вы не можете добавлять сообщения в очередь.",
                );
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при поиске сообщения: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете добавлять сообщения в очередь.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

#[get("/track/{recipient_id}")]
pub async fn track_email(recipient_id: web::Path<i32>, pool: web::Data<DbPool>) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match set_email_recipient_opened_status(&mut conn, recipient_id.into_inner(), true) {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/assets/placeholder.png"))
            .finish(),
        Err(err) => {
            error!("Database connection error: {}", err); // Log the error for debugging
            HttpResponse::InternalServerError().finish()
        }
    }
}
