use actix_web::{HttpRequest, HttpResponse, get};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::frontend::open_frontend_html;

#[get("/na")]
pub async fn not_assigned(request: HttpRequest, _user: AuthenticatedUser) -> HttpResponse {
    match open_frontend_html("assets/dist/app/no-access.html").await {
        Ok(file) => file.into_response(&request),
        Err(error) => {
            log::error!("Failed to open no-access frontend document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
