use actix_session::Session;
use actix_web::http::header;
use actix_web::{HttpResponse, Responder, get, post, web};
use log::error;
use tera::Context;

use crate::TEMPLATES;
use crate::db::DbPool;
use crate::forms::recipients::{
    AddGroupForm, AddRecipientForm, AssignGroupRecipientForm, DeleteGroupForm, DeleteRecipientForm,
};
use crate::models::alert::{add_flash_message, get_flash_messages};
use crate::models::auth::AuthenticatedUser;
use crate::repository::recipient::{
    assign_recipient_to_group, clean_all_recipients_and_groups, create_group, create_recipient,
    delete_group, delete_recipient, get_hub_all_recipients, get_hub_group_recipients,
    get_hub_nogroup_recipients, unassign_recipient_from_group,
};

#[get("/recipients")]
pub async fn recipients(
    user: AuthenticatedUser,
    mut session: Session,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "recipients");

    if let Some(hub_id) = user.0.hub_id {
        if let Ok(recipients) = get_hub_nogroup_recipients(&mut conn, hub_id) {
            context.insert("no_group_recipients", &recipients);
        }
        if let Ok(recipients) = get_hub_all_recipients(&mut conn, hub_id) {
            context.insert("recipients", &recipients);
        }
        if let Ok(groups) = get_hub_group_recipients(&mut conn, hub_id) {
            context.insert("groups", &groups);
        }
    }

    HttpResponse::Ok().body(
        TEMPLATES
            .render("recipients/recipients.html", &context)
            .unwrap_or_default(),
    )
}

#[post("/recipients/add")]
pub async fn recipients_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(hub_id) = user.0.hub_id {
        match create_recipient(&mut conn, hub_id, &form) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Получатель успешно добавлен.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при создании получателя: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете добавлять получателей.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/delete")]
pub async fn recipients_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
        match delete_recipient(&mut conn, form.id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Получатель удален.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении получателя: {}", err),
                );
            }
        }
    } else {
        add_flash_message(&mut session, "danger", "Вы не можете удалять получателей.");
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/group/add")]
pub async fn recipients_group_add(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AddGroupForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
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
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/group/delete")]
pub async fn recipients_group_delete(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<DeleteGroupForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
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
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/assign")]
pub async fn recipients_assign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
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
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/unassign")]
pub async fn recipients_unassign(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    web::Form(form): web::Form<AssignGroupRecipientForm>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(_) = user.0.hub_id {
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
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}

#[post("/recipients/clean")]
pub async fn recipients_clean(
    user: AuthenticatedUser,
    pool: web::Data<DbPool>,
    mut session: Session,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(err) => {
            add_flash_message(&mut session, "danger", "Ошибка сервера. Попробуйте позже.");
            error!("Database connection error: {}", err); // Log the error for debugging
            return HttpResponse::InternalServerError().finish();
        }
    };

    if let Some(hub_id) = user.0.hub_id {
        match clean_all_recipients_and_groups(&mut conn, hub_id) {
            Ok(_) => {
                add_flash_message(&mut session, "success", "Все получатели и группы удалены.");
            }
            Err(err) => {
                add_flash_message(
                    &mut session,
                    "danger",
                    &format!("Ошибка при удалении групп и получателей: {}", err),
                );
            }
        }
    } else {
        add_flash_message(
            &mut session,
            "danger",
            "Вы не можете удалять группы и получатели.",
        );
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/recipients"))
        .finish()
}
