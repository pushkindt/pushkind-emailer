use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use serde::Deserialize;

use crate::domain::recipient::UpdateRecipient;

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

impl From<SaveRecipientForm> for UpdateRecipient {
    fn from(form: SaveRecipientForm) -> Self {
        let fields = form
            .field
            .iter()
            .cloned()
            .zip(form.value.iter().cloned())
            .collect();

        let unsubscribed_at = if form.active {
            None
        } else {
            Some(chrono::Utc::now().naive_utc())
        };

        Self {
            name: form.name,
            email: form.email,
            fields,
            unsubscribed_at,
            groups: form.groups,
        }
    }
}
