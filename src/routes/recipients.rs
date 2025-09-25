use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, ensure_role, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use serde::Deserialize;
use tera::{Context, Tera};

use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SourceRecipientForm, UploadRecipientsForm,
};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::recipients::{RecipientModalData, RecipientsService};

#[derive(Deserialize)]
struct RecipientsQueryParams {
    q: Option<String>,
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

    let service = RecipientsService::new(repo.get_ref());
    let page = params.page.unwrap_or(1);
    let data = match service.load_overview(user.hub_id, page, params.q.clone()) {
        Ok(data) => data,
        Err(err) => {
            log::error!("Failed to get recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "recipients",
        &common_config.auth_service_url,
    );
    context.insert("crm_service_url", &server_config.crm_service_url);
    context.insert("recipients", &data.recipients);
    if let Some(search) = data.search_query {
        context.insert("search_query", &search);
    }

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

    let service = RecipientsService::new(repo.get_ref());
    match service.create_recipient(user.hub_id, form) {
        Ok(_) => FlashMessage::success("Получатель успешно добавлен.").send(),
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при добавлении получателя.").send();
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

    let service = RecipientsService::new(repo.get_ref());
    match service.delete_recipient(user.hub_id, form) {
        Ok(_) => {
            FlashMessage::success("Получатель удален.").send();
            redirect("/recipients")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to delete recipient: {err}");
            FlashMessage::error("Ошибка при удалении получателя.").send();
            redirect("/recipients")
        }
    }
}

#[post("/recipients/clean")]
pub async fn recipients_clean(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = RecipientsService::new(repo.get_ref());
    match service.clean(user.hub_id) {
        Ok(_) => {
            FlashMessage::success("Все группы удалены.").send();
            FlashMessage::success("Все получатели удалены.").send();
            redirect("/recipients")
        }
        Err(err) => {
            log::error!("Failed to clean recipients: {err}");
            FlashMessage::error("Ошибка при удалении получателей.").send();
            redirect("/recipients")
        }
    }
}

#[post("/recipients/upload")]
pub async fn recipients_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadRecipientsForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = RecipientsService::new(repo.get_ref());
    match service.upload_recipients(user.hub_id, form) {
        Ok(_) => {
            FlashMessage::success("Получатели добавлены.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(format!("Ошибка при парсинге получателей: {message}")).send();
            redirect("/recipients")
        }
        Err(err) => {
            log::error!("Failed to add recipients: {err}");
            FlashMessage::error("Ошибка при добавлении получателей.").send();
            redirect("/recipients")
        }
    }
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

    let service = RecipientsService::new(repo.get_ref());
    let data = match service.load_modal(user.hub_id, recipient_id.into_inner()) {
        Ok(data) => data,
        Err(ServiceError::NotFound) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Error retrieving recipient: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    render_recipient_modal(&data, &tera)
}

fn render_recipient_modal(data: &RecipientModalData, tera: &Tera) -> HttpResponse {
    let mut context = Context::new();
    context.insert("recipient", &data.recipient);
    context.insert("groups", &data.groups);

    render_template(tera, "recipients/modal_body.html", &context)
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

    let service = RecipientsService::new(repo.get_ref());
    match service.save_recipient(user.hub_id, &form) {
        Ok(_) => {
            FlashMessage::success("Получатель сохранён.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при обработке формы.").send();
            redirect("/recipients")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Error saving recipient: {err}");
            FlashMessage::error("Ошибка при сохранении получателя.").send();
            redirect("/recipients")
        }
    }
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

    let id_cookie = match req.cookie("id") {
        Some(cookie) => cookie,
        None => {
            log::error!("No id cookie found");
            return redirect("/recipients");
        }
    };

    let service = RecipientsService::new(repo.get_ref());
    match service
        .import_from_source(user.hub_id, form, id_cookie.value())
        .await
    {
        Ok(_) => {
            FlashMessage::success("Получатели успешно добавлены.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(format!("Ошибка при загрузке получателей: {message}")).send();
            redirect("/recipients")
        }
        Err(err) => {
            log::error!("Failed to create recipients: {err}");
            FlashMessage::error("Ошибка при добавлении получателя.").send();
            redirect("/recipients")
        }
    }
}
