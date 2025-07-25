use std::collections::HashMap;
use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv;
use reqwest::header::COOKIE;
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
pub struct SourceRecipientForm {
    pub source: String, // URL of the service to fetch a JSON array of NewRecipient
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
    /// Parses the uploaded CSV into a list of [`NewRecipient`] values.
    ///
    /// The file must contain a header row with at least `name` and `email`
    /// columns.  A `groups` column can specify comma separated group names and
    /// any additional columns are treated as custom fields for the recipient.
    ///
    /// # Errors
    /// Returns [`UploadRecipientsFormError`] when the file cannot be read or the
    /// CSV data is invalid.
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
                        if field.is_empty() {
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

#[derive(Debug, Error)]
pub enum SourceRecipientFormError {
    #[error("Error reading api")]
    RequestError,
    #[error("Error parsing api")]
    DeserializeError,
}

impl From<reqwest::Error> for SourceRecipientFormError {
    fn from(_: reqwest::Error) -> Self {
        Self::RequestError
    }
}

impl SourceRecipientForm {
    pub async fn load(
        &self,
        id_value: &str,
    ) -> Result<Vec<NewRecipient>, SourceRecipientFormError> {
        let client = reqwest::Client::new();
        let response = client
            .get(&self.source)
            .header(COOKIE, format!("id={}", id_value))
            .send()
            .await?;

        if response.status().is_success() {
            let recipients: Vec<NewRecipient> = response.json().await?;
            Ok(recipients)
        } else {
            Err(SourceRecipientFormError::RequestError)
        }
    }
}
