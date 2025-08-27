use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use serde::Deserialize;
use tera::{Context, Tera};
use validator::Validate;

use crate::domain::recipient::NewRecipient;
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, SourceRecipientForm,
    UploadRecipientsForm,
};
use crate::models::config::ServerConfig;
use crate::repository::{
    DieselRepository, GroupListQuery, GroupReader, GroupWriter, RecipientListQuery,
    RecipientReader, RecipientWriter,
};

#[derive(Deserialize)]
struct RecipientsQueryParams {
    page: Option<usize>,
}

#[get("/recipients")]
pub async fn recipients_show(
    params: web::Query<RecipientsQueryParams>,
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    common_config: web::Data<CommonServerConfig>,
    server_config: web::Data<ServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "recipients",
        &common_config.auth_service_url,
    );
    context.insert("crm_service_url", &server_config.crm_service_url);

    let page = params.page.unwrap_or(1);
    let query = RecipientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let recipients = match repo.list_recipients(query) {
        Ok((total, recipients)) => {
            Paginated::new(recipients, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
        }
        Err(err) => {
            log::error!("Failed to get recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("recipients", &recipients);

    render_template(&tera, "recipients/recipients.html", &context)
}

#[post("/recipients/add")]
pub async fn recipients_add(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    if form.validate().is_err() {
        FlashMessage::error("Ошибка при добавлении получателя.").send();
        return redirect("/recipients");
    }

    let mut new_recipient: NewRecipient = form.into();

    new_recipient.hub_id = user.hub_id;
    match repo.create_recipients(&[new_recipient]) {
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
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<DeleteRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let recipient = match repo.get_recipient_by_id(form.id, user.hub_id) {
        Ok(Some(recipient)) => recipient.recipient,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving recipient: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match repo.delete_recipient(recipient.id) {
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
pub async fn recipients_clean(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    match repo.delete_all_groups(user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Все группы удалены.").send();
        }
        Err(err) => {
            log::error!("Failed to delete groups: {err}");
            FlashMessage::error("Ошибка при удалении групп.").send();
            return redirect("/recipients");
        }
    }

    match repo.delete_all_recipients(user.hub_id) {
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
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadRecipientsForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let recipients: Vec<NewRecipient> = match form.parse(user.hub_id) {
        Ok(recipients) => recipients,
        Err(err) => {
            FlashMessage::error(format!("Ошибка при парсинге получателей: {err}")).send();
            return redirect("/recipients");
        }
    };

    match repo.create_recipients(&recipients) {
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
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut context = Context::new();

    let recipient_id = recipient_id.into_inner();

    let recipient = match repo.get_recipient_by_id(recipient_id, user.hub_id) {
        Ok(Some(recipient)) => recipient,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving recipient: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let query = GroupListQuery::new(user.hub_id);

    let groups = match repo.list_groups(query) {
        Ok((_total, groups)) => groups,
        Err(e) => {
            log::error!("Error retrieving groups: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("recipient", &recipient);
    context.insert("groups", &groups);

    render_template(&tera, "recipients/modal_body.html", &context)
}

#[post("/recipients/save")]
pub async fn recipients_save(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let form: SaveRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/recipients");
        }
    };

    let recipient = match repo.get_recipient_by_id(form.id, user.hub_id) {
        Ok(Some(recipient)) => recipient.recipient,
        Ok(None) => {
            log::error!("Recipient not found");
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving recipient: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match repo.update_recipient(recipient.id, &form.into()) {
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
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<SourceRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    if form.validate().is_err() {
        FlashMessage::error("Ошибка валидации формы.").send();
        return redirect("/recipients");
    }

    let id_cookie = match req.cookie("id") {
        Some(cookie) => cookie,
        None => {
            log::error!("No id cookie found");
            return redirect("/recipients");
        }
    };

    let new_recipients: Vec<NewRecipient> = match form.load(id_cookie.value()).await {
        Ok(recipients) => recipients,
        Err(err) => {
            log::error!("Failed to load recipients: {err}");
            FlashMessage::error("Ошибка при загрузке получателей.").send();
            return redirect("/recipients");
        }
    };

    match repo.create_recipients(&new_recipients) {
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
