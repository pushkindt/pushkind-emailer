use std::io::Read;

use actix_multipart::form::MultipartForm;
use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::TEMPLATES;
use crate::db::{DbPool, get_db_connection};
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, UploadRecipientsForm,
};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::repository::recipient::{
    clean_all_recipients_and_groups, create_recipient, delete_recipient, get_hub_all_groups,
    get_hub_all_recipients, get_recipient, get_recipient_fields, get_recipient_group_ids,
    save_recipient, update_recipients_from_csv,
};

#[get("/recipients")]
pub async fn recipients(
    user: AuthenticatedUser,
    mut session: Session,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let flash_messages = get_flash_messages(&mut session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");

    if let Some(hub_id) = user.0.hub_id {
        if let Ok(recipients) = get_hub_all_recipients(&mut conn, hub_id) {
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
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if let Some(hub_id) = user.0.hub_id {
        match create_recipient(&mut conn, hub_id, &form.name, &form.email) {
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
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
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

#[post("/recipients/clean")]
pub async fn recipients_clean(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if let Some(hub_id) = user.0.hub_id {
        match clean_all_recipients_and_groups(&mut conn, hub_id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Все получатели и группы удалены.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении групп и получателей: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете удалять группы и получатели.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/upload")]
pub async fn recipients_upload(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    MultipartForm(mut form): MultipartForm<UploadRecipientsForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let mut csv_content = String::new();

    if let Some(hub_id) = user.0.hub_id {
        match form.csv.file.read_to_string(&mut csv_content) {
            Ok(_) => match update_recipients_from_csv(&mut conn, hub_id, &csv_content) {
                Ok(_) => {
                    add_flash_message(&mut session, "success", "Файл успешно загружен.");
                }
                Err(err) => {
                    add_flash_message(
                        &mut session,
                        "danger",
                        &format!("Ошибка при загрузке файла: {}", err),
                    );
                }
            },
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при чтении файла: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете загружать группы и получатели.",
        );
    }
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/modal/{recipient_id}")]
pub async fn recipients_modal(
    recipient_id: web::Path<i32>,
    _user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let mut context = Context::new();

    let recipient_id = recipient_id.into_inner();

    if let Ok(recipient) = get_recipient(&mut conn, recipient_id) {
        context.insert("recipient", &recipient);

        if let Ok(fields) = get_recipient_fields(&mut conn, recipient_id) {
            context.insert("recipient_fields", &fields);
        }
        if let Ok(groups) = get_recipient_group_ids(&mut conn, recipient_id) {
            context.insert("recipient_groups", &groups);
        }
        if let Ok(groups) = get_hub_all_groups(&mut conn, recipient.hub_id) {
            context.insert("groups", &groups);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("recipients/modal_body.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/recipients/save")]
pub async fn recipients_save(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    form: web::Bytes,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let form: SaveRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            add_flash_message(
                &mut session,
                "danger",
                &format!("Ошибка при обработке формы: {}", err),
            );
            return HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/recipients"))
                .finish();
        }
    };

    if user.0.hub_id.is_some() {
        let fields = form.field.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        let values = form.value.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        match save_recipient(
            &mut conn,
            form.id,
            &form.name,
            &form.email,
            form.active,
            &form.groups,
            &fields,
            &values,
        ) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Получатель сохранён.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при сохранении получателя: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете сохранять получателей.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}
