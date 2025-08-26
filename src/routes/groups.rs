use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, render_template};
use pushkind_common::routes::{ensure_role, redirect};
use tera::{Context, Tera};
use validator::Validate;

use crate::domain::group::NewGroup;
use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::repository::{DieselRepository, GroupReader, GroupWriter, RecipientReader};

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

    let recipients = match repo.list_recipients(user.hub_id) {
        Ok(recipients) => recipients,
        Err(err) => {
            log::error!("Error while listing recipients: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let groups = match repo.list_groups(user.hub_id) {
        Ok(groups) => groups,
        Err(err) => {
            log::error!("Error while listing groups: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let custom_fields = match repo.list_custom_fields(user.hub_id) {
        Ok(custom_fields) => custom_fields,
        Err(err) => {
            log::error!("Error while listing custom fields: {err}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    let mut context = base_context(
        &flash_messages,
        &user,
        "groups",
        &server_config.auth_service_url,
    );
    context.insert("groups", &groups);
    context.insert("custom_fields", &custom_fields);
    context.insert("recipients", &recipients);

    render_template(&tera, "groups/groups.html", &context)
}

#[post("/groups/add")]
pub async fn groups_add(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddGroupForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    if form.validate().is_err() {
        FlashMessage::error("Некорректные данные.").send();
        return redirect("/groups");
    }

    let new_group = NewGroup {
        hub_id: user.hub_id,
        name: &form.name,
    };

    match repo.create_group(&new_group) {
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
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<DeleteGroupForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group = match repo.get_group_by_id(form.id, user.hub_id) {
        Ok(Some(group)) => group,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving group: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match repo.delete_group(group.group.id) {
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
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group = match repo.get_group_by_id(form.group_id, user.hub_id) {
        Ok(Some(group)) => group,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving group: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match repo.assign_recipient_to_group(group.group.id, form.recipient_id) {
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
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let group = match repo.get_group_by_id(form.group_id, user.hub_id) {
        Ok(Some(group)) => group,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving group: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    match repo.unassign_recipient_to_group(group.group.id, form.recipient_id) {
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
    repo: web::Data<DieselRepository>,
    tera: web::Data<Tera>,
) -> impl Responder {
    if ensure_role(&user, "emailer", Some("/na")).is_err() {
        return HttpResponse::Unauthorized().finish();
    };

    let mut context = Context::new();

    let group_id = group_id.into_inner();

    let group = match repo.get_group_by_id(group_id, user.hub_id) {
        Ok(Some(group)) => group,
        Ok(None) => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            log::error!("Error retrieving group: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    context.insert("group", &group);

    render_template(&tera, "groups/modal_body.html", &context)
}
