use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use tera::Context;

use crate::domain::recipient::NewRecipient;
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, UploadRecipientsForm,
};
use crate::repository::group::DieselGroupRepository;
use crate::repository::recipient::{
    DieselRecipientRepository, create_recipient, delete_recipient, get_hub_all_recipients,
};
use crate::repository::{GroupReader, GroupWriter, RecipientReader, RecipientWriter};
use crate::routes::render_template;

#[get("/recipients")]
pub async fn recipients_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");
    context.insert("home_url", &server_config.auth_service_url);

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

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
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

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().finish(),
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

    let recipient_repo = DieselRecipientRepository::new(&pool);
    let group_repo = DieselGroupRepository::new(&pool);

    match group_repo.delete_all(user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Все группы удалены.").send();
        }
        Err(err) => {
            log::error!("Failed to delete groups: {err}");
            FlashMessage::error("Ошибка при удалении групп.").send();
            return redirect("/recipients");
        }
    }

    match recipient_repo.delete_all(user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Все получатели удалены.").send();
        }
        Err(err) => {
            log::error!("Failed to delete recipients: {err}");
            FlashMessage::error("Ошибка при удалении получателей.").send();
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

    let recipient_repo = DieselRecipientRepository::new(&pool);

    let recipients: Vec<NewRecipient> = match form.parse(user.hub_id) {
        Ok(recipients) => recipients,
        Err(err) => {
            FlashMessage::error(format!("Ошибка при парсинге получателей: {err}")).send();
            return redirect("/recipients");
        }
    };

    match recipient_repo.create(&recipients) {
        Ok(_) => {
            FlashMessage::success("Получатели добавлены.").send();
        }
        Err(err) => {
            log::error!("Failed to add clients: {err}");
            FlashMessage::error("Ошибка при добавлении получателей.").send();
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

    let recipient_repo = DieselRecipientRepository::new(&pool);
    let group_repo = DieselGroupRepository::new(&pool);

    let mut context = Context::new();

    let recipient_id = recipient_id.into_inner();

    let recipient = match recipient_repo.get_by_id(recipient_id) {
        Ok(Some(recipient)) => recipient,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving recipient: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let groups = match group_repo.list(user.hub_id) {
        Ok(groups) => groups,
        Err(e) => {
            log::error!("Error retrieving groups: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("recipient", &recipient);
    context.insert("groups", &groups);

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

    let recipient_repo = DieselRecipientRepository::new(&pool);

    let form: SaveRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/recipients");
        }
    };

    match recipient_repo.update(form.id, &form.into()) {
        Ok(_) => {
            FlashMessage::success("Получатель сохранён.").send();
        }
        Err(err) => {
            log::error!("Error saving recipient: {err}");
            FlashMessage::error("Ошибка при сохранении получателя.").send();
        }
    }

    redirect("/recipients")
}
