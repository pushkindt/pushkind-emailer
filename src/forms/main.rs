use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile, text::Text};
use serde::Deserialize;

use crate::{domain::email::NewEmail, utils::read_attachment_file};

/// Form data for sending a new email with optional attachment.
#[derive(MultipartForm)]
pub struct SendEmailForm {
    pub message: Text<String>,
    pub subject: Text<Option<String>>,
    #[multipart(limit = "10MB")]
    pub attachment: Option<TempFile>,
    pub recipients: MpJson<Vec<String>>,
}

/// Form data to remove an existing email.
#[derive(Deserialize)]
pub struct DeleteEmailForm {
    pub id: i32,
}

impl From<SendEmailForm> for NewEmail {
    /// Converts a [`SendEmailForm`] into the domain [`NewEmail`] type.
    fn from(mut form: SendEmailForm) -> Self {
        let (attachment_name, attachment_mime, attachment) =
            if let Some(attachment) = form.attachment.as_mut() {
                match read_attachment_file(attachment) {
                    Ok((name, mime, data)) => (name, mime, data),
                    Err(err) => {
                        log::error!("error reading file: {err}");
                        (None, None, None)
                    }
                }
            } else {
                (None, None, None)
            };

        Self {
            hub_id: 0,
            message: form.message.0,
            subject: form.subject.0,
            attachment,
            attachment_mime,
            attachment_name,
            recipients: form.recipients.0,
        }
    }
}
