//! Recipient-related form types and input validation.
use std::collections::BTreeMap;
use std::io::Read;

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use reqwest::header::COOKIE;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::recipient::{NewRecipient, UpdateRecipient};
use crate::domain::types::{GroupId, HubId, RecipientEmail, RecipientName, RecipientSourceUrl};
use crate::forms::FormError;

/// Form for adding a single recipient manually.
#[derive(Deserialize, Validate)]
pub struct AddRecipientForm {
    #[validate(length(min = 1, message = "Укажите имя."))]
    pub name: String,
    #[validate(email(message = "Укажите корректный электронный адрес."))]
    pub email: String,
}

pub struct AddRecipientPayload {
    pub name: RecipientName,
    pub email: RecipientEmail,
}

/// Form specifying an external source to load recipients from.
#[derive(Deserialize, Validate)]
pub struct SourceRecipientForm {
    #[validate(url(message = "Укажите корректный URL."))]
    pub source: String, // URL of the service to fetch a JSON array of NewRecipient
}

pub struct SourceRecipientPayload {
    pub source: RecipientSourceUrl,
}

/// Form for uploading a CSV file containing recipients.
#[derive(MultipartForm)]
pub struct UploadRecipientsForm {
    #[multipart(limit = "10MB")]
    pub csv: TempFile,
}

/// Form data for updating an existing recipient.
#[derive(Deserialize, Validate)]
pub struct SaveRecipientForm {
    #[validate(length(min = 1, message = "Укажите имя."))]
    pub name: String,
    #[validate(email(message = "Укажите корректный электронный адрес."))]
    pub email: String,
    #[serde(default)]
    pub groups: Vec<i32>,
    #[serde(default)]
    pub field: Vec<String>,
    #[serde(default)]
    pub value: Vec<String>,
}

pub struct SaveRecipientPayload {
    pub name: RecipientName,
    pub email: RecipientEmail,
    pub groups: Vec<GroupId>,
    pub fields: BTreeMap<String, String>,
}

impl TryFrom<AddRecipientForm> for AddRecipientPayload {
    type Error = FormError;

    fn try_from(form: AddRecipientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;
        Ok(Self {
            name: RecipientName::new(form.name).map_err(|_| FormError::InvalidName)?,
            email: RecipientEmail::new(form.email).map_err(|_| FormError::InvalidEmail)?,
        })
    }
}

impl AddRecipientPayload {
    pub fn into_domain(self, hub_id: HubId) -> NewRecipient {
        NewRecipient {
            name: self.name,
            email: self.email,
            hub_id,
            fields: None,
            groups: None,
        }
    }
}

impl TryFrom<SaveRecipientForm> for SaveRecipientPayload {
    type Error = FormError;

    fn try_from(form: SaveRecipientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        if form.field.len() != form.value.len() {
            return Err(FormError::InvalidField);
        }

        let fields = form.field.into_iter().zip(form.value).collect();

        Ok(Self {
            name: RecipientName::new(form.name).map_err(|_| FormError::InvalidName)?,
            email: RecipientEmail::new(form.email).map_err(|_| FormError::InvalidEmail)?,
            groups: form
                .groups
                .into_iter()
                .map(GroupId::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| FormError::InvalidGroupId)?,
            fields,
        })
    }
}

impl SaveRecipientPayload {
    pub fn into_domain(self) -> UpdateRecipient {
        UpdateRecipient {
            name: self.name,
            email: self.email,
            fields: self.fields,
            groups: self.groups,
        }
    }
}

impl TryFrom<SourceRecipientForm> for SourceRecipientPayload {
    type Error = FormError;

    fn try_from(form: SourceRecipientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            source: RecipientSourceUrl::new(form.source).map_err(|_| FormError::InvalidSource)?,
        })
    }
}

/// Errors returned when loading recipients from a remote service.
#[derive(Debug, Error)]
pub enum SourceRecipientFormError {
    #[error("Ошибка при обращении к API.")]
    RequestError,
    #[error("Ошибка при разборе ответа API.")]
    DeserializeError,
    #[error("Ошибка валидации получателей: {0}")]
    ValidationError(String),
}

impl From<reqwest::Error> for SourceRecipientFormError {
    fn from(_: reqwest::Error) -> Self {
        Self::RequestError
    }
}

impl SourceRecipientPayload {
    /// Loads recipients from the external service specified in [`SourceRecipientForm`].
    ///
    /// The `id_value` is sent as a cookie named `id` with the request.
    pub async fn load(
        &self,
        id_value: &str,
        hub_id: HubId,
    ) -> Result<Vec<NewRecipient>, SourceRecipientFormError> {
        let client = reqwest::Client::new();
        let response = client
            .get(self.source.as_str())
            .header(COOKIE, format!("id={id_value}"))
            .send()
            .await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct SourceRecipient {
                name: String,
                email: Option<String>,
                fields: Option<BTreeMap<String, String>>,
                groups: Option<Vec<String>>,
            }

            let recipients: Vec<SourceRecipient> = response.json().await?;
            let recipients = recipients
                .into_iter()
                .filter(|r| r.email.is_some())
                .filter(|r| !r.email.as_ref().unwrap().trim().is_empty())
                .map(|r| {
                    NewRecipient::try_new(
                        r.name,
                        r.email.unwrap(),
                        hub_id.get(),
                        r.fields,
                        r.groups,
                    )
                    .map_err(|err| SourceRecipientFormError::ValidationError(err.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(recipients)
        } else {
            Err(SourceRecipientFormError::RequestError)
        }
    }
}

/// Errors that can occur while processing an uploaded CSV of recipients.
#[derive(Debug, Error)]
pub enum UploadRecipientsFormError {
    #[error("Ошибка при чтении CSV файла.")]
    FileReadError,
    #[error("Ошибка при разборе CSV файла.")]
    CsvParseError,
    #[error("Ошибка валидации получателей: {0}")]
    ValidationError(String),
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
            let mut optional_fields = BTreeMap::new();

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

            let recipient =
                NewRecipient::try_new(name, email, hub_id, Some(optional_fields), Some(groups))
                    .map_err(|err| UploadRecipientsFormError::ValidationError(err.to_string()))?;
            recipients.push(recipient);
        }

        Ok(recipients)
    }
}

#[cfg(test)]
mod tests {
    use super::{SaveRecipientForm, SaveRecipientPayload};

    #[test]
    fn save_recipient_rejects_mismatched_fields() {
        let form = SaveRecipientForm {
            name: "Jane".to_string(),
            email: "jane@example.com".to_string(),
            groups: vec![],
            field: vec!["city".to_string(), "role".to_string()],
            value: vec!["Paris".to_string()],
        };

        let result: Result<SaveRecipientPayload, _> = form.try_into();
        assert!(result.is_err());
    }
}
