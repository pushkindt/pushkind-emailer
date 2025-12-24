//! Group-related HTTP handlers.
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use tera::{Context, Tera};

use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm};
use crate::repository::DieselRepository;
use crate::services::groups::{
    assign_recipients, create_group, delete_group, load_group_modal, load_groups_overview,
};

#[get("/groups")]
pub async fn groups_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let data = match load_groups_overview(&user, repo.get_ref()) {
        Ok(data) => data,
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(err) => {
            log::error!("Error while loading groups overview: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "groups",
        &server_config.auth_service_url,
    );
    context.insert("groups", &data.groups);
    context.insert("custom_fields", &data.custom_fields);
    context.insert("recipients", &data.recipients);

    render_template(&tera, "groups/groups.html", &context)
}

#[post("/group/add")]
pub async fn group_add(
    web::Form(form): web::Form<AddGroupForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match create_group(form, &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Группа успешно добавлена.").send();
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Некорректные данные.").send();
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return redirect("/na");
        }
        Err(err) => {
            log::error!("Error while creating group: {err}");
            FlashMessage::error("Ошибка при создании группы.").send();
        }
    }

    redirect("/groups")
}

#[post("/group/{group_id}/delete")]
pub async fn group_delete(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match delete_group(group_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Группа удалена.").send();
            redirect("/groups")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Error while deleting group: {err}");
            FlashMessage::error("Ошибка при удалении группы.").send();
            redirect("/groups")
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
            log::error!("Error parsing form: {err}");
            FlashMessage::error("Ошибка при обработке формы.").send();
            return redirect("/groups");
        }
    };

    match assign_recipients(group_id.into_inner(), form, &user, repo.get_ref()) {
        Ok(_) => {
            FlashMessage::success("Группа назначена получателю.").send();
            redirect("/groups")
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при обработке формы.").send();
            redirect("/groups")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Error while assigning group: {err}");
            FlashMessage::error("Ошибка при назначении группы.").send();
            redirect("/groups")
        }
    }
}

#[post("/group/{group_id}/modal")]
pub async fn group_modal(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let group_id = group_id.into_inner();
    let group = match load_group_modal(group_id, &user, repo.get_ref()) {
        Ok(group) => group,
        Err(ServiceError::NotFound) => return HttpResponse::NotFound().finish(),
        Err(ServiceError::Unauthorized) => return HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Error retrieving group: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = Context::new();
    context.insert("group", &group);

    render_template(&tera, "groups/modal_body.html", &context)
}
