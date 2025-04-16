use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile, text::Text};
use serde::Deserialize;

#[derive(MultipartForm)]
pub struct SendEmailForm {
    pub message: Text<String>,
    pub subject: Text<Option<String>>,
    #[multipart(limit = "10MB")]
    pub attachment: TempFile,
    pub recipients: MpJson<Vec<String>>,
}

#[derive(Deserialize)]
pub struct DeleteEmailForm {
    pub id: i32,
}
