use std::io::Read;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use tera::Context;

use crate::db::{DbPool, get_db_connection};
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, UploadRecipientsForm,
};
use crate::models::auth::AuthenticatedUser;
use crate::repository::recipient::{
    clean_all_recipients_and_groups, create_recipient, delete_recipient, get_hub_all_groups,
    get_hub_all_recipients, get_recipient, get_recipient_fields, get_recipient_group_ids,
    save_recipient, update_recipients_from_csv,
};
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};

#[get("/recipients")]
pub async fn recipients(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");

    if let Ok(recipients) = get_hub_all_recipients(&mut conn, user.hub_id) {
        context.insert("recipients", &recipients);
    }

    render_template("recipients/recipients.html", &context)
}

#[post("/recipients/add")]
pub async fn recipients_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match create_recipient(&mut conn, user.hub_id, &form.name, &form.email) {
        Ok(_) => {
            FlashMessage::success("Получатель успешно добавлен.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при создании получателя: {}", err)).send();
        }
    }

    redirect("/recipients")
}

#[post("/recipients/delete")]
pub async fn recipients_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match delete_recipient(&mut conn, form.id) {
        Ok(_) => {
            FlashMessage::success("Получатель удален.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при удалении получателя: {}", err)).send();
        }
    }

    redirect("/recipients")
}

#[post("/recipients/clean")]
pub async fn recipients_clean(user: AuthenticatedUser, pool: web::Data<DbPool>) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match clean_all_recipients_and_groups(&mut conn, user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Все получатели и группы удалены.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при удалении групп и получателей: {}", err)).send();
        }
    }

    redirect("/recipients")
}

#[post("/recipients/upload")]
pub async fn recipients_upload(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    MultipartForm(mut form): MultipartForm<UploadRecipientsForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let mut csv_content = String::new();

    match form.csv.file.read_to_string(&mut csv_content) {
        Ok(_) => match update_recipients_from_csv(&mut conn, user.hub_id, &csv_content) {
            Ok(_) => {
                FlashMessage::success("Файл успешно загружен.").send();
            }
            Err(err) => {
                FlashMessage::error(format!("Ошибка при загрузке файла: {}", err)).send();
            }
        },
        Err(err) => {
            FlashMessage::error(format!("Ошибка при чтении файла: {}", err)).send();
        }
    }

    redirect("/recipients")
}

#[post("/recipients/modal/{recipient_id}")]
pub async fn recipients_modal(
    recipient_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

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

    render_template("recipients/modal_body.html", &context)
}

#[post("/recipients/save")]
pub async fn recipients_save(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    form: web::Bytes,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let form: SaveRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            FlashMessage::error(format!("Ошибка при обработке формы: {}", err)).send();
            return redirect("/recipients");
        }
    };

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
            FlashMessage::success("Получатель сохранён.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при сохранении получателя: {}", err)).send();
        }
    }

    redirect("/recipients")
}
