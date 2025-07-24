use std::collections::HashMap;
use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::recipient::{NewRecipient, UpdateRecipient};

#[derive(Deserialize, Validate)]
pub struct AddRecipientForm {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
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

#[derive(Debug, Error)]
pub enum UploadRecipientsFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
}

impl From<std::io::Error> for UploadRecipientsFormError {
    fn from(_: std::io::Error) -> Self {
        UploadRecipientsFormError::FileReadError
    }
}

impl From<csv::Error> for UploadRecipientsFormError {
    fn from(_: csv::Error) -> Self {
        UploadRecipientsFormError::CsvParseError
    }
}

impl UploadRecipientsForm {
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewRecipient>, UploadRecipientsFormError> {
        let mut csv_content = String::new();
        self.csv.file.read_to_string(&mut csv_content)?;

        let mut rdr = csv::Reader::from_reader(csv_content.as_bytes());

        let mut recipients: Vec<NewRecipient> = Vec::new();

        let headers = rdr.headers()?.clone();

        for result in rdr.records() {
            let record = result?;
            let mut optional_fields = HashMap::new();

            let mut name = String::new();
            let mut email = String::new();
            let mut groups = Vec::new();

            for (i, field) in record.iter().enumerate() {
                match headers.get(i) {
                    Some("name") => name = field.to_string(),
                    Some("email") => email = field.to_string(),
                    Some("groups") => {
                        groups = field
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    Some(header) => {
                        if field.len() == 0 {
                            continue;
                        }
                        optional_fields.insert(header.to_string(), field.to_string());
                    }
                    None => continue,
                }
            }

            recipients.push(NewRecipient {
                name,
                email,
                hub_id,
                groups: Some(groups),
                fields: Some(optional_fields),
            });
        }

        Ok(recipients)
    }
}

impl From<AddRecipientForm> for NewRecipient {
    fn from(form: AddRecipientForm) -> Self {
        Self {
            name: form.name,
            email: form.email,
            hub_id: 0,
            groups: None,
            fields: None,
        }
    }
}
