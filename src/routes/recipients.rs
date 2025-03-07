use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::forms::recipients::{AddRecipientForm, DeleteRecipientForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::repository::recipient::{create_recipient, delete_recipient, get_hub_recipients};

#[get("/recipients")]
pub async fn recipients(
    user: AuthenticatedUser,
    mut session: Session,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");

    if let Some(hub_id) = user.0.hub_id {
        if let Ok(recipients) = get_hub_recipients(&mut conn, hub_id) {
            context.insert("recipients", &recipients);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("recipients/recipients.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/recipients/add")]
pub async fn recipients_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddRecipientForm>,
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

    if let Some(hub_id) = user.0.hub_id {
        match create_recipient(&mut conn, hub_id, &form) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Получатель успешно добавлен.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при создании получателя: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете добавлять получателей.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/delete")]
pub async fn recipients_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteRecipientForm>,
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
        match delete_recipient(&mut conn, form.id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Получатель удален.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении получателя: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете удалять получателей.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}
