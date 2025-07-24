use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::db::DbPool;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{alert_level_to_str, ensure_role, redirect};
use tera::Context;
use validator::Validate;

use crate::domain::group::NewGroup;
use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::repository::group::DieselGroupRepository;
use crate::repository::recipient::DieselRecipientRepository;
use crate::repository::{GroupReader, GroupWriter, RecipientReader};
use crate::routes::render_template;

#[get("/groups")]
pub async fn groups(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group_repo = DieselGroupRepository::new(&pool);
    let recipient_repo = DieselRecipientRepository::new(&pool);

    let recipients = match recipient_repo.list(user.hub_id) {
        Ok(recipients) => recipients,
        Err(err) => {
            log::error!("Error while listing recipients: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let groups = match group_repo.list(user.hub_id) {
        Ok(groups) => groups,
        Err(err) => {
            log::error!("Error while listing groups: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };
    let custom_fields = match recipient_repo.list_custom_fields(user.hub_id) {
        Ok(custom_fields) => custom_fields,
        Err(err) => {
            log::error!("Error while listing custom fields: {}", err);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let alerts = flash_messages
        .iter()
        .map(|f| (f.content(), alert_level_to_str(&f.level())))
        .collect::<Vec<_>>();
    let mut context = Context::new();
    context.insert("alerts", &alerts);
    context.insert("current_user", &user);
    context.insert("current_page", "groups");
    context.insert("home_url", &server_config.auth_service_url);
    context.insert("groups", &groups);
    context.insert("custom_fields", &custom_fields);
    context.insert("recipients", &recipients);

    render_template("groups/groups.html", &context)
}

#[post("/groups/add")]
pub async fn groups_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddGroupForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    if form.validate().is_err() {
        FlashMessage::error("Некорректные данные.").send();
        return redirect("/groups");
    }

    let group_repo = DieselGroupRepository::new(&pool);

    let new_group = NewGroup {
        hub_id: user.hub_id,
        name: &form.name,
    };

    match group_repo.create(&new_group) {
        Ok(_) => {
            FlashMessage::success("Группа успешно добавлена.").send();
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
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteGroupForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group_repo = DieselGroupRepository::new(&pool);

    match group_repo.delete(form.id) {
        Ok(_) => {
            FlashMessage::success("Группа удалена.").send();
        }
        Err(err) => {
            log::error!("Error while deleting group: {err}");
            FlashMessage::error("Ошибка при удалении группы.").send();
        }
    }

    redirect("/groups")
}

#[post("/groups/assign")]
pub async fn groups_assign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group_repo = DieselGroupRepository::new(&pool);

    match group_repo.assign_recipient(form.group_id, form.recipient_id) {
        Ok(_) => {
            FlashMessage::success("Группа назначена получателю.").send();
        }
        Err(err) => {
            log::error!("Error while assigning group: {err}");
            FlashMessage::error("Ошибка при назначении группы.").send();
        }
    }

    redirect("/groups")
}

#[post("/groups/unassign")]
pub async fn groups_unassign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group_repo = DieselGroupRepository::new(&pool);

    match group_repo.unassign_recipient(form.group_id, form.recipient_id) {
        Ok(_) => {
            FlashMessage::success("Назначение группы удалено.").send();
        }
        Err(err) => {
            log::error!("Error while unassigning group: {err}");
            FlashMessage::error("Ошибка при удалении назначения группы.").send();
        }
    }

    redirect("/groups")
}

#[post("/groups/modal/{group_id}")]
pub async fn groups_modal(
    group_id: web::Path<i32>,
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if ensure_role(&user, "emailer", Some("/na")).is_err() {
        return HttpResponse::Unauthorized().finish();
    };

    let group_repo = DieselGroupRepository::new(&pool);

    let mut context = Context::new();

    let group_id = group_id.into_inner();

    let group = match group_repo.get_by_id(group_id) {
        Ok(Some(group)) => group,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving recipient: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("group", &group);

    render_template("groups/modal_body.html", &context)
}
