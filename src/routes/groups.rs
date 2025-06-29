use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use tera::Context;

use crate::db::{DbPool, get_db_connection};
use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::models::auth::AuthenticatedUser;
use crate::models::config::ServerConfig;
use crate::repository::recipient::{
    assign_recipient_to_group, create_group, delete_group, get_hub_all_recipients,
    get_hub_all_recipients_fields, get_hub_group_recipients, unassign_recipient_from_group,
};
use crate::routes::{alert_level_to_str, ensure_role, redirect, render_template};

#[get("/groups")]
pub async fn groups(
    user: AuthenticatedUser,
    flash_messages: IncomingFlashMessages,
    pool: web::Data<DbPool>,
    server_config: web::Data<ServerConfig>,
) -> impl Responder {
    if let Err(response) = ensure_role(&user, "emailer", Some("/na")) {
        return response;
    };

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
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

    if let Ok(recipients) = get_hub_all_recipients(&mut conn, user.hub_id) {
        context.insert("recipients", &recipients);
    }
    if let Ok(groups) = get_hub_group_recipients(&mut conn, user.hub_id) {
        context.insert("groups", &groups);
    }
    if let Ok(custom_fields) = get_hub_all_recipients_fields(&mut conn, user.hub_id) {
        context.insert("custom_fields", &custom_fields);
    }

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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match create_group(&mut conn, user.hub_id, &form.name) {
        Ok(_) => {
            FlashMessage::success("Группа успешно добавлена.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при создании группы: {}", err)).send();
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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match delete_group(&mut conn, form.id) {
        Ok(_) => {
            FlashMessage::success("Группа удалена.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при удалении группы: {}", err)).send();
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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match assign_recipient_to_group(&mut conn, form.recipient_id, form.group_id) {
        Ok(_) => {
            FlashMessage::success("Группа назначена получателю.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при назначении группы: {}", err)).send();
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

    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    match unassign_recipient_from_group(&mut conn, form.recipient_id, form.group_id) {
        Ok(_) => {
            FlashMessage::success("Назначение группы удалено.").send();
        }
        Err(err) => {
            FlashMessage::error(format!("Ошибка при удалении назначения группы: {}", err)).send();
        }
    }

    redirect("/groups")
}
