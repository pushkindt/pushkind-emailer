//! Group-related HTTP handlers.
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use tera::{Context, Tera};

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
    let data = match load_groups_overview(repo.get_ref(), &user) {
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

#[post("/groups/add")]
pub async fn groups_add(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<crate::forms::groups::AddGroupForm>,
) -> impl Responder {
    match create_group(repo.get_ref(), &user, form) {
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

#[post("/groups/delete")]
pub async fn groups_delete(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<crate::forms::groups::DeleteGroupForm>,
) -> impl Responder {
    match delete_group(repo.get_ref(), &user, form) {
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

#[post("/groups/assign")]
pub async fn groups_assign(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    form: web::Bytes,
) -> impl Responder {
    match assign_recipients(repo.get_ref(), &user, &form) {
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

#[post("/groups/modal/{group_id}")]
pub async fn groups_modal(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let group_id = group_id.into_inner();
    let group = match load_group_modal(repo.get_ref(), &user, group_id) {
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
