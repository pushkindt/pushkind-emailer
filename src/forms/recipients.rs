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

#[derive(Deserialize)]
pub struct AddGroupForm {
    pub name: String,
}

#[derive(Deserialize)]
pub struct DeleteGroupForm {
    pub id: i32,
}

#[derive(Deserialize)]
pub struct AssignGroupRecipientForm {
    pub recipient_id: i32,
    pub group_id: i32,
}

#[derive(MultipartForm)]
pub struct UploadRecipientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}
