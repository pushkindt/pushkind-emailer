//! Recipient-related HTTP handlers.
use actix_multipart::form::MultipartForm;
use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use tera::{Context, Tera};

use crate::dto::recipients::RecipientsQueryParams;
use crate::forms::recipients::{
    AddRecipientForm, SaveRecipientForm, SourceRecipientForm, UploadRecipientsForm,
};
use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::services::recipients::{
    clean_recipients, create_recipient, delete_recipient, import_recipients_from_source,
    load_recipient_modal, load_recipients_overview, save_recipient, upload_recipients,
};

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
    let data = match load_recipients_overview(params.into_inner(), &user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
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

#[post("/recipient/add")]
pub async fn recipient_add(
    web::Form(form): web::Form<AddRecipientForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match create_recipient(form, &user, repo.get_ref()) {
        Ok(_) => FlashMessage::success("Получатель успешно добавлен.").send(),
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при добавлении получателя.").send();
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(err) => {
            log::error!("Failed to create recipient: {err}");
            FlashMessage::error("Ошибка при создании получателя.").send();
        }
    }

    redirect("/recipients")
}

#[post("/recipient/{recipient_id}/delete")]
pub async fn recipients_delete(
    recipient_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_recipient(recipient_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Получатель удален.").send();
            redirect("/recipients")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
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
    match clean_recipients(&user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Все группы удалены.").send();
            FlashMessage::success("Все получатели удалены.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
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
    MultipartForm(form): MultipartForm<UploadRecipientsForm>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match upload_recipients(form, &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Получатели добавлены.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(format!("Ошибка при парсинге получателей: {message}")).send();
            redirect("/recipients")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to add recipients: {err}");
            FlashMessage::error("Ошибка при добавлении получателей.").send();
            redirect("/recipients")
        }
    }
}

#[post("/recipient/{recipient_id}/modal")]
pub async fn recipient_modal(
    recipient_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let data = match load_recipient_modal(recipient_id.into_inner(), &user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(ServiceError::NotFound) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Error retrieving recipient: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = Context::new();
    context.insert("recipient", &data.recipient);
    context.insert("groups", &data.groups);

    render_template(&tera, "recipients/modal_body.html", &context)
}

#[post("/recipient/{recipient_id}/save")]
pub async fn recipient_save(
    recipient_id: web::Path<i32>,
    form: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form: SaveRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/recipients");
        }
    };

    match save_recipient(recipient_id.into_inner(), form, &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Получатель сохранён.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при обработке формы.").send();
            redirect("/recipients")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Error saving recipient: {err}");
            FlashMessage::error("Ошибка при сохранении получателя.").send();
            redirect("/recipients")
        }
    }
}

#[post("/recipients/source")]
pub async fn recipients_source(
    web::Form(form): web::Form<SourceRecipientForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    req: HttpRequest,
) -> impl Responder {
    let id_cookie = match req.cookie("id") {
        Some(cookie) => cookie,
        None => {
            log::error!("No id cookie found");
            return redirect("/recipients");
        }
    };

    match import_recipients_from_source(form, &user, repo.get_ref(), id_cookie.value()).await {
        Ok(_) => {
            FlashMessage::success("Получатели успешно добавлены.").send();
            redirect("/recipients")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(format!("Ошибка при загрузке получателей: {message}")).send();
            redirect("/recipients")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to create recipients: {err}");
            FlashMessage::error("Ошибка при добавлении получателя.").send();
            redirect("/recipients")
        }
    }
}
