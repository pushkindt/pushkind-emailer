//! Group-related HTTP handlers.
use actix_files::NamedFile;
use actix_web::{Either, HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{check_role, redirect};
use pushkind_common::services::errors::ServiceError;

use crate::SERVICE_ACCESS_ROLE;
use crate::dto::api::{ApiMutationErrorDto, ApiMutationSuccessDto};
use crate::forms::groups::{
    AddGroupForm, AddGroupPayload, AssignGroupRecipientForm, AssignGroupRecipientPayload,
};
use crate::frontend::open_frontend_html;
use crate::repository::DieselRepository;
use crate::services::groups::{assign_recipients, create_group, delete_group};

#[get("/groups")]
pub async fn groups_show(user: AuthenticatedUser) -> Either<NamedFile, HttpResponse> {
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Either::Right(redirect("/na"));
    }

    match open_frontend_html("assets/dist/app/groups.html").await {
        Ok(file) => Either::Left(file),
        Err(err) => {
            log::error!("Failed to open Emailer groups document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

#[post("/group/add")]
pub async fn group_add(
    web::Form(form): web::Form<AddGroupForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let payload: AddGroupPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    match create_group(payload, &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Группа успешно добавлена.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Error while creating group: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при создании группы.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/group/{group_id}/delete")]
pub async fn group_delete(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_group(group_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Группа удалена.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Группа не найдена.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Error while deleting group: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при удалении группы.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}

#[post("/group/{group_id}/assign")]
pub async fn group_assign(
    group_id: web::Path<i32>,
    form: web::Bytes,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let form: AssignGroupRecipientForm = match serde_html_form::from_bytes(&form) {
        Ok(form) => form,
        Err(err) => {
            log::error!("Error parsing group assignment form: {err}");
            return HttpResponse::BadRequest().json(ApiMutationErrorDto {
                message: "Ошибка при обработке формы.".into(),
                field_errors: Vec::new(),
            });
        }
    };

    let payload: AssignGroupRecipientPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(err) => {
            return HttpResponse::BadRequest().json(ApiMutationErrorDto::from(&err));
        }
    };

    match assign_recipients(group_id.into_inner(), payload, &user, repo.get_ref()) {
        Ok(_) => HttpResponse::Ok().json(ApiMutationSuccessDto {
            message: "Группа назначена получателям.".into(),
            redirect_to: None,
        }),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().json(ApiMutationErrorDto {
            message: "Группа не найдена.".into(),
            field_errors: Vec::new(),
        }),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().json(ApiMutationErrorDto {
            message: "Недостаточно прав.".into(),
            field_errors: Vec::new(),
        }),
        Err(err) => {
            log::error!("Error while assigning group: {err}");
            HttpResponse::InternalServerError().json(ApiMutationErrorDto {
                message: "Ошибка при назначении группы.".into(),
                field_errors: Vec::new(),
            })
        }
    }
}
