use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::forms::main::{DeleteEmailForm, SendEmailForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::models::zmq::ZmqConfig;
use crate::repository::email::{
    create_email, get_email_by_id, get_user_all_emails_with_recipients, remove_email,
};
use crate::repository::recipient::{get_hub_all_groups, get_hub_all_recipients};
use crate::utils::send_zmq_email_id;

#[get("/")]
pub async fn index(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
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
    zmq_config: web::Data<ZmqConfig>,
    form: web::Bytes,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    let form = String::from_utf8_lossy(&form);

    let form: SendEmailForm = match serde_html_form::from_str(&form) {
        Ok(form) => form,
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при обработке формы: {}", err),
            );
            return HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/"))
                .finish();
        }
    };

    if let Some(_) = user.0.hub_id {
        match create_email(&mut conn, &form, user.0.id) {
            Ok(email) => {
                match send_zmq_email_id(email.id, &zmq_config) {
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
                }
                add_flash_message(&mut session, "success", "Сообщение создано.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при создании сообщения: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете отправлять сообщения.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

#[post("/delete_email")]
pub async fn delete_email(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteEmailForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
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
    zmq_config: web::Data<ZmqConfig>,
    web::Form(form): web::Form<DeleteEmailForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
        match get_email_by_id(&mut conn, form.id) {
            Ok(email) if email.user_id == user.0.id => {
                match send_zmq_email_id(email.id, &zmq_config) {
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
