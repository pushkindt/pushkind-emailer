use std::fs;
use std::io::Result;

use actix_files::Directory;
use actix_multipart::form::MultipartForm;
use actix_session::Session;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::{FromRequest, Responder, post};
use actix_web::{HttpRequest, HttpResponse, body::BoxBody, dev::ServiceResponse};
use futures_util::future::FutureExt; // for `.now_or_never()`
use tera::Context;
use uuid::Uuid;

use crate::TEMPLATES;
use crate::forms::files::UploadFileForm;
use crate::models::alert::add_flash_message;
use crate::models::auth::AuthenticatedUser;

pub fn file_manager(dir: &Directory, req: &HttpRequest) -> Result<ServiceResponse<BoxBody>> {
    let mut payload = Payload::None;
    let auth_user_result = AuthenticatedUser::from_request(req, &mut payload).now_or_never();

    let user = match auth_user_result {
        Some(Ok(user)) => user,
        Some(Err(err)) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Auth error: {}", err),
            ));
        }
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Extractor not ready",
            ));
        }
    };

    let hub_id = match user.0.hub_id {
        Some(hub_id) => format!("{}/", hub_id),
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Auth error: You are not allowed to view files"),
            ));
        }
    };

    let full_path = dir.base.join(&dir.path).join(&hub_id);

    let mut entries = match fs::read_dir(&full_path) {
        Ok(read_dir) => read_dir
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().into_string())
            .filter_map(|e| e.ok())
            .collect(),
        Err(_) => vec![],
    };

    entries.sort();

    let mut context = Context::new();
    context.insert("current_user", &user);
    context.insert("current_page", "files");
    context.insert("entries", &entries);
    context.insert("hub_id", &hub_id);

    let response = HttpResponse::Ok().body(
        TEMPLATES
            .render("files/files.html", &context)
            .unwrap_or_default(),
    );
    let req = req.clone();

    Ok(ServiceResponse::new(req, response))
}

#[post("/upload_image")]
pub async fn upload_image(
    user: AuthenticatedUser,
    MultipartForm(form): MultipartForm<UploadFileForm>,
    mut session: Session,
) -> impl Responder {
    let hub_id = match user.0.hub_id {
        Some(hub_id) => hub_id,
        None => {
            add_flash_message(&mut session, "danger", "Вы не можете загружать файлы.");
            return HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/upload"))
                .finish();
        }
    };

    let file_name = form
        .image
        .file_name
        .unwrap_or(format!("upload-{}", Uuid::new_v4()));

    let filepath = format!("./upload/{}/{}", hub_id, file_name);

    match form.image.file.persist(filepath) {
        Ok(_) => add_flash_message(&mut session, "danger", "Файл успешно загружен."),
        Err(_) => add_flash_message(&mut session, "danger", "Ошибка при загрузке файла."),
    }

    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/upload"))
        .finish()
}
