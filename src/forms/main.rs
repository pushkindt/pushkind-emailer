//! Email composition and sending form types.
use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile, text::Text};

/// Form data for sending a new email with optional attachment.
#[derive(MultipartForm)]
pub struct SendEmailForm {
    pub message: Text<String>,
    pub subject: Text<Option<String>>,
    pub cooldown_days: Text<Option<i64>>,
    #[multipart(limit = "10MB")]
    pub attachment: Option<TempFile>,
    pub recipients: MpJson<Vec<String>>,
}
