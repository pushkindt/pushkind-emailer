use std::collections::HashMap;
use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use reqwest::header::COOKIE;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::recipient::{NewRecipient, UpdateRecipient};

/// Form for adding a single recipient manually.
#[derive(Deserialize, Validate)]
pub struct AddRecipientForm {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
}

/// Form specifying an external source to load recipients from.
#[derive(Deserialize, Validate)]
pub struct SourceRecipientForm {
    #[validate(url)]
    pub source: String, // URL of the service to fetch a JSON array of NewRecipient
}

/// Form used to delete a recipient by identifier.
#[derive(Deserialize)]
pub struct DeleteRecipientForm {
    pub id: i32,
}

/// Form for uploading a CSV file containing recipients.
#[derive(MultipartForm)]
pub struct UploadRecipientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

/// Form data for updating an existing recipient.
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

/// Errors that can occur while processing an uploaded CSV of recipients.
#[derive(Debug, Error)]
pub enum UploadRecipientsFormError {
    #[error("Error reading CSV file")]
    FileReadError,
    #[error("Error parsing CSV file")]
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
                    Some("name") => name = field.trim().to_string(),
                    Some("email") => email = field.trim().to_string(),
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

            if name.is_empty() || email.is_empty() {
                // Skip records missing required fields.
                continue;
            }

            recipients.push(NewRecipient::new(
                name,
                email,
                hub_id,
                Some(optional_fields),
                Some(groups),
            ));
        }

        Ok(recipients)
    }
}

impl From<AddRecipientForm> for NewRecipient {
    fn from(form: AddRecipientForm) -> Self {
        NewRecipient::new(form.name, form.email, 0, None, None)
    }
}

/// Errors returned when loading recipients from a remote service.
#[derive(Debug, Error)]
pub enum SourceRecipientFormError {
    #[error("Error reading API")]
    RequestError,
    #[error("Error parsing API")]
    DeserializeError,
}

impl From<reqwest::Error> for SourceRecipientFormError {
    fn from(_: reqwest::Error) -> Self {
        Self::RequestError
    }
}

impl SourceRecipientForm {
    /// Loads recipients from the external service specified in [`SourceRecipientForm`].
    ///
    /// The `id_value` is sent as a cookie named `id` with the request.
    pub async fn load(
        &self,
        id_value: &str,
    ) -> Result<Vec<NewRecipient>, SourceRecipientFormError> {
        let client = reqwest::Client::new();
        let response = client
            .get(&self.source)
            .header(COOKIE, format!("id={id_value}"))
            .send()
            .await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct SourceRecipient {
                name: String,
                email: Option<String>,
                hub_id: i32,
                fields: Option<HashMap<String, String>>,
                groups: Option<Vec<String>>,
            }

            let recipients: Vec<SourceRecipient> = response.json().await?;
            let recipients = recipients
                .into_iter()
                .filter(|r| r.email.is_some())
                .filter(|r| !r.email.as_ref().unwrap().trim().is_empty())
                .map(|r| NewRecipient::new(r.name, r.email.unwrap(), r.hub_id, r.fields, r.groups))
                .collect();
            Ok(recipients)
        } else {
            Err(SourceRecipientFormError::RequestError)
        }
    }
}
