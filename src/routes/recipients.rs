//! Recipient-related HTTP handlers.
use actix_files::NamedFile;
use actix_multipart::form::MultipartForm;
use actix_web::{Either, HttpRequest, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::dto::mutation::{ApiMutationErrorDto, ApiMutationSuccessDto};
use pushkind_common::routes::{check_role, redirect};
use pushkind_common::services::errors::ServiceError;

use crate::SERVICE_ACCESS_ROLE;
use crate::forms::recipients::{
    AddRecipientForm, AddRecipientPayload, SaveRecipientForm, SaveRecipientPayload,
    SourceRecipientForm, SourceRecipientPayload, UploadRecipientsForm,
};
use crate::frontend::open_frontend_html;
use crate::repository::DieselRepository;
use crate::services::recipients::{
    clean_recipients, create_recipient, delete_recipient, import_recipients_from_source,
    save_recipient, upload_recipients,
};

#[get("/recipients")]
pub async fn recipients_show(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Either::Right(redirect("/na"));
    }

    match open_frontend_html("assets/dist/app/recipients.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer recipients document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

#[post("/recipient/add")]
pub async fn recipient_add(
    web::Form(form): web::Form<AddRecipientForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddRecipientPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    match create_recipient(payload, &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Получатель успешно добавлен.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to create recipient: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при создании получателя.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/recipient/{recipient_id}/delete")]
pub async fn recipients_delete(
    recipient_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_recipient(recipient_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Получатель удалён.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Получатель не найден.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to delete recipient: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при удалении получателя.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/recipients/clean")]
pub async fn recipients_clean(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match clean_recipients(&user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Все получатели и группы удалены.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to clean recipients: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при удалении получателей.".into(),
                field_errors: Vec::new(),
            })
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
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Получатели добавлены.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Form(message)) => HttpResponse::BadRequest().json(ApiMutationErrorDto {
            message: format!("Ошибка при парсинге получателей: {message}"),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to add recipients: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при добавлении получателей.".into(),
                field_errors: Vec::new(),
            })
        }
    }
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
            log::error!("Error parsing recipient save form: {err}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto {
                message: "Ошибка при обработке формы.".into(),
                field_errors: Vec::new(),
            });
        }
    };

    let payload: SaveRecipientPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    match save_recipient(recipient_id.into_inner(), payload, &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Получатель сохранён.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Получатель не найден.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Error saving recipient: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при сохранении получателя.".into(),
                field_errors: Vec::new(),
            })
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
    let payload: SourceRecipientPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    let id_cookie = match req.cookie("id") {
        Some(cookie) => cookie,
        None => {
            log::error!("No id cookie found");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto {
                message: "Не удалось получить данные текущей сессии.".into(),
                field_errors: Vec::new(),
            });
        }
    };

    match import_recipients_from_source(payload, &user, repo.get_ref(), id_cookie.value()).await {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Получатели успешно добавлены.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Form(message)) => HttpResponse::BadRequest().json(ApiMutationErrorDto {
            message: format!("Ошибка при загрузке получателей: {message}"),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to create recipients: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при добавлении получателей.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}
