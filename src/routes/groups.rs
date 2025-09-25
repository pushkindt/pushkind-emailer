use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, ensure_role, redirect, render_template};
use pushkind_common::services::errors::ServiceError;
use tera::{Context, Tera};

use crate::repository::DieselRepository;
use crate::services::groups::{GroupsOverviewData, GroupsService};

#[get("/groups")]
pub async fn groups_show(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = GroupsService::new(repo.get_ref());
    let data = match service.load_overview(user.hub_id) {
        Ok(data) => data,
        Err(err) => {
            log::error!("Error while loading groups overview: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    render_groups_overview(
        data,
        flash_messages,
        user,
        &server_config.auth_service_url,
        &tera,
    )
}

fn render_groups_overview(
    data: GroupsOverviewData,
    flash_messages: IncomingFlashMessages,
    user: AuthenticatedUser,
    auth_service_url: &str,
    tera: &Tera,
) -> HttpResponse {
    let mut context = base_context(&flash_messages, &user, "groups", auth_service_url);
    context.insert("groups", &data.groups);
    context.insert("custom_fields", &data.custom_fields);
    context.insert("recipients", &data.recipients);

    render_template(tera, "groups/groups.html", &context)
}

#[post("/groups/add")]
pub async fn groups_add(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<crate::forms::groups::AddGroupForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = GroupsService::new(repo.get_ref());
    match service.create_group(user.hub_id, form) {
        Ok(_) => {
            FlashMessage::success("Группа успешно добавлена.").send();
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Некорректные данные.").send();
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
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = GroupsService::new(repo.get_ref());
    match service.delete_group(user.hub_id, form) {
        Ok(_) => {
            FlashMessage::success("Группа удалена.").send();
            redirect("/groups")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
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
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let service = GroupsService::new(repo.get_ref());
    match service.assign_recipients(user.hub_id, &form) {
        Ok(_) => {
            FlashMessage::success("Группа назначена получателю.").send();
            redirect("/groups")
        }
        Err(ServiceError::Form(_)) => {
            FlashMessage::error("Ошибка при обработке формы.").send();
            redirect("/groups")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
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
    if ensure_role(&user, "emailer", Some("/na")).is_err() {
        return HttpResponse::Unauthorized().finish();
    };

    let service = GroupsService::new(repo.get_ref());
    let group_id = group_id.into_inner();
    let group = match service.load_modal(user.hub_id, group_id) {
        Ok(group) => group,
        Err(ServiceError::NotFound) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Error retrieving group: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = Context::new();
    context.insert("group", &group);

    render_template(&tera, "groups/modal_body.html", &context)
}
