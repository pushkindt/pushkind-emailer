use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddRecipientForm {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct DeleteRecipientForm {
    pub id: i32,
}

#[derive(MultipartForm)]
pub struct UploadRecipientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

#[derive(Deserialize)]
pub struct SaveRecipientForm {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub groups: Vec<i32>,
    #[serde(default)]
    pub field: Vec<String>,
    #[serde(default)]
    pub value: Vec<String>,
}
