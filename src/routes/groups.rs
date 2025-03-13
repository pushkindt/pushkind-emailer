use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use tera::Context;

use crate::TEMPLATES;
use crate::db::{DbPool, get_db_connection};
use crate::forms::recipients::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::repository::recipient::{
    assign_recipient_to_group, create_group, delete_group, get_hub_all_recipients,
    get_hub_all_recipients_fields, get_hub_group_recipients, unassign_recipient_from_group,
};

#[get("/groups")]
pub async fn groups(
    user: AuthenticatedUser,
    mut session: Session,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    let flash_messages = get_flash_messages(&mut session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "groups");

    if let Some(hub_id) = user.0.hub_id {
        if let Ok(recipients) = get_hub_all_recipients(&mut conn, hub_id) {
            context.insert("recipients", &recipients);
        }
        if let Ok(groups) = get_hub_group_recipients(&mut conn, hub_id) {
            context.insert("groups", &groups);
        }
        if let Ok(custom_fields) = get_hub_all_recipients_fields(&mut conn, hub_id) {
            context.insert("custom_fields", &custom_fields);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("groups/groups.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/groups/add")]
pub async fn groups_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddGroupForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if let Some(hub_id) = user.0.hub_id {
        match create_group(&mut conn, hub_id, &form) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Группа успешно добавлена.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при создании группы: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете добавлять группы.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/groups"))
        .finish()
}

#[post("/groups/delete")]
pub async fn groups_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteGroupForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
        match delete_group(&mut conn, form.id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Группа удалена.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении группы: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете удалять группы.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/groups"))
        .finish()
}

#[post("/groups/assign")]
pub async fn groups_assign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
        match assign_recipient_to_group(&mut conn, &form) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Группа назначена получателю.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при назначении группы: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете назначать группы.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/groups"))
        .finish()
}

#[post("/groups/unassign")]
pub async fn groups_unassign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match get_db_connection(&pool) {
        Some(conn) => conn,
        None => return HttpResponse::InternalServerError().finish(),
    };

    if user.0.hub_id.is_some() {
        match unassign_recipient_from_group(&mut conn, &form) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Назначение группы удалено.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении назначения группы: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете удалять назначения группы.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/groups"))
        .finish()
}
