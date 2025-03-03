use actix_web::{HttpResponse, Responder, get};

use tera::Context;

use crate::TEMPLATES;
use crate::routes::auth::AuthenticatedUser;

#[get("/")]
pub async fn index(user: AuthenticatedUser) -> impl Responder {
    let mut context = Context::new();
    context.insert("current_user", &user);
    HttpResponse::Ok().body(
        TEMPLATES
            .render("main/index.html", &context)
            .unwrap_or_default(),
    )
}
