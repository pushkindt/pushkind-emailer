//! Main email workflow HTTP handlers.
use std::error::Error;
use std::sync::Arc;

use actix_files::NamedFile;
use actix_multipart::form::MultipartForm;
use actix_web::{Either, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{check_role, redirect};
use pushkind_common::services::errors::ServiceError;
use pushkind_common::zmq::ZmqSender;

use crate::SERVICE_ACCESS_ROLE;
use crate::dto::api::{ApiMutationErrorDto, ApiMutationSuccessDto};
use crate::dto::main::ExportedEmailRecipients;
use crate::forms::main::SendEmailForm;
use crate::frontend::open_frontend_html;
use crate::repository::DieselRepository;
use crate::services::main::{
    delete_email as delete_email_service,
    export_email_recipients as export_email_recipients_service, mark_email_opened,
    queue_email_retry, queue_new_email,
};

#[get("/")]
pub async fn index(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Either::Right(redirect("/na"));
    }

    match open_frontend_html("assets/dist/app/index.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer index document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

#[post("/email/send")]
pub async fn send_email(
    user: AuthenticatedUser,
    form: Result<MultipartForm<SendEmailForm>, Box<dyn Error>>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form = match form {
        Ok(form) => form.0,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto {
                message: format!("Ошибка при обработке формы: {err}"),
                field_errors: Vec::new(),
            });
        }
    };

    match queue_new_email(form, &user, repo.get_ref(), zmq_sender.as_ref()).await {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Сообщение добавлено в очередь.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Form(message)) => HttpResponse::BadRequest().json(ApiMutationErrorDto {
            message,
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to queue email: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при добавлении сообщения в очередь.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/email/{email_id}/delete")]
pub async fn delete_email(
    email_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_email_service(email_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Сообщение удалено.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Сообщение не найдено.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to delete email: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при удалении сообщения.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/email/{email_id}/resend")]
pub async fn resend_email(
    email_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
) -> impl Responder {
    match queue_email_retry(
        email_id.into_inner(),
        &user,
        repo.get_ref(),
        zmq_sender.as_ref(),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Сообщение добавлено в очередь.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Сообщение не найдено.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Failed to queue retry: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при повторной отправке сообщения.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[get("/track/{recipient_id}")]
pub async fn track_email(
    recipient_id: web::Path<i32>,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let recipient_id = recipient_id.into_inner();

    match mark_email_opened(recipient_id, repo.get_ref()) {
        Ok(_) => redirect("/assets/placeholder.png"),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to update recipient status for {recipient_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/email/{email_id}/recipients/export")]
pub async fn export_email_recipients(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    email_id: web::Path<i32>,
) -> impl Responder {
    let email_id = email_id.into_inner();

    match export_email_recipients_service(email_id, &user, repo.get_ref()) {
        Ok(ExportedEmailRecipients { filename, bytes }) => HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(bytes),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => redirect("/na"),
        Err(err) => {
            log::error!("Failed to export recipients: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
