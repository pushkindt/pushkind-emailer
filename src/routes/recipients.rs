use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use tera::Context;
use validator::Validate;

use crate::domain::recipient::NewRecipient;
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, SourceRecipientForm,
    UploadRecipientsForm,
};
use crate::models::config::ServerConfig;
use crate::repository::group::DieselGroupRepository;
use crate::repository::recipient::DieselRecipientRepository;
use crate::repository::{GroupReader, GroupWriter, RecipientReader, RecipientWriter};
use crate::routes::render_template;

#[get("/recipients")]
pub async fn recipients_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let recipient_repo = DieselRecipientRepository::new(&pool);

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");
    context.insert("home_url", &common_config.auth_service_url);
    context.insert("crm_service_url", &server_config.crm_service_url);

    let recipients = match recipient_repo.list(user.hub_id) {
        Ok(recipients) => recipients,
        Err(err) => {
            log::error!("Failed to get recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("recipients", &recipients);

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

    if form.validate().is_err() {
        FlashMessage::error("Ошибка при добавлении получателя.").send();
        return redirect("/recipients");
    }

    let recipient_repo = DieselRecipientRepository::new(&pool);

    let mut new_recipient: NewRecipient = form.into();

    new_recipient.hub_id = user.hub_id;
    match recipient_repo.create(&[new_recipient]) {
        Ok(_) => {
            FlashMessage::success("Получатель успешно добавлен.").send();
        }
        Err(err) => {
            log::error!("Failed to create recipient: {err}");
            FlashMessage::error("Ошибка при создании получателя.").send();
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

    let recipient_repo = DieselRecipientRepository::new(&pool);

    match recipient_repo.delete(form.id) {
        Ok(_) => {
            FlashMessage::success("Получатель удален.").send();
        }
        Err(err) => {
            log::error!("Failed to delete recipient: {err}");
            FlashMessage::error("Ошибка при удалении получателя.").send();
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

#[post("/recipients/source")]
pub async fn recipients_source(
    req: HttpRequest,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<SourceRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let id_cookie = match req.cookie("id") {
        Some(cookie) => cookie,
        None => {
            log::error!("No id cookie found");
            return redirect("/recipients");
        }
    };

    let recipient_repo = DieselRecipientRepository::new(&pool);

    let new_recipients: Vec<NewRecipient> = match form.load(id_cookie.value()).await {
        Ok(recipients) => recipients,
        Err(err) => {
            log::error!("Failed to load recipients: {err}");
            FlashMessage::error("Ошибка при загрузке получателей.").send();
            return redirect("/recipients");
        }
    };

    match recipient_repo.create(&new_recipients) {
        Ok(_) => {
            FlashMessage::success("Получатели успешно добавлены.").send();
        }
        Err(err) => {
            log::error!("Failed to create recipients: {err}");
            FlashMessage::error("Ошибка при добавлении получателя.").send();
        }
    }

    redirect("/recipients")
}
