use actix_session::Session;
use actix_web::{HttpResponse, Responder, get};
use tera::Context;

use crate::TEMPLATES;
use crate::models::alert::get_flash_messages;
use crate::models::auth::AuthenticatedUser;

#[get("/")]
pub async fn index(user: AuthenticatedUser, session: Session) -> impl Responder {
    let flash_messages = get_flash_messages(&session);
    let mut context = Context::new();
    context.insert("alerts", &flash_messages);
    context.insert("current_user", &user);
    context.insert("current_page", "index");
    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/index.html", &context)
            .unwrap_or_default(),
    )
}
