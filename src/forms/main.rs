use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile, text::Text};
use serde::Deserialize;

use crate::{domain::email::NewEmail, utils::read_attachment_file};

#[derive(MultipartForm)]
pub struct SendEmailForm {
    pub message: Text<String>,
    pub subject: Text<Option<String>>,
    #[multipart(limit = "10MB")]
    pub attachment: Option<TempFile>,
    pub recipients: MpJson<Vec<String>>,
}

#[derive(Deserialize)]
pub struct DeleteEmailForm {
    pub id: i32,
}

impl From<SendEmailForm> for NewEmail {
    fn from(mut form: SendEmailForm) -> Self {
        let (attchment_name, attachement_mime, attachment) =
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
            attachment_mime: attachement_mime,
            attachment_name: attchment_name,
            recipients: form.recipients.0,
        }
    }
}
