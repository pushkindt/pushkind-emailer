//! Web form types used by the application.
//!
//! Each submodule contains structs representing data submitted by the user via
//! HTTP forms or multipart requests.
use std::borrow::Cow;

use thiserror::Error;
use validator::{ValidationError, ValidationErrors};

pub mod groups;
pub mod main;
pub mod recipients;
pub mod settings;

/// Enumerates all possible form field validation errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

#[derive(Debug, Error)]
pub enum FormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),

    #[error("Логин заполнен некорректно.")]
    InvalidLogin,

    #[error("Пароль заполнен некорректно.")]
    InvalidPassword,

    #[error("Отправитель заполнен некорректно.")]
    InvalidSender,

    #[error("SMTP сервер заполнен некорректно.")]
    InvalidSmtpServer,

    #[error("SMTP порт заполнен некорректно.")]
    InvalidSmtpPort,

    #[error("IMAP сервер заполнен некорректно.")]
    InvalidImapServer,

    #[error("IMAP порт заполнен некорректно.")]
    InvalidImapPort,

    #[error("Дата заполнена некорректно.")]
    InvalidCreatedAt,

    #[error("Сообщение заполнено некорректно.")]
    InvalidMessage,

    #[error("Укажите имя.")]
    InvalidName,

    #[error("Укажите корректный электронный адрес.")]
    InvalidEmail,

    #[error("Укажите корректный URL.")]
    InvalidSource,

    #[error("Идентификатор заполнен некорректно.")]
    InvalidId,

    #[error("CSV файл заполнен некорректно.")]
    InvalidCsv,

    #[error("Группы заполнены некорректно.")]
    InvalidGroups,

    #[error("Дополнительные поля заполнены некорректно.")]
    InvalidField,

    #[error("Значения дополнительных полей заполнены некорректно.")]
    InvalidValue,

    #[error("Выберите получателей.")]
    InvalidRecipientId,

    #[error("Группа заполнена некорректно.")]
    InvalidGroupId,

    #[error("Тема заполнена некорректно.")]
    InvalidSubject,

    #[error("Количество дней заполнено некорректно.")]
    InvalidCooldownDays,

    #[error("Вложение заполнено некорректно.")]
    InvalidAttachment,

    #[error("Получатели заполнены некорректно.")]
    InvalidRecipients,

    #[error("Изображение заполнено некорректно.")]
    InvalidImage,
}

impl FormError {
    #[allow(dead_code)]
    pub(crate) fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            _ => self
                .field()
                .map(|field| vec![field_error(field, self.to_string())])
                .unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    fn field(&self) -> Option<&'static str> {
        match self {
            Self::Validation(_) => None,
            Self::InvalidLogin => Some("login"),
            Self::InvalidPassword => Some("password"),
            Self::InvalidSender => Some("sender"),
            Self::InvalidSmtpServer => Some("smtp_server"),
            Self::InvalidSmtpPort => Some("smtp_port"),
            Self::InvalidImapServer => Some("imap_server"),
            Self::InvalidImapPort => Some("imap_port"),
            Self::InvalidCreatedAt => Some("created_at"),
            Self::InvalidMessage => Some("message"),
            Self::InvalidName => Some("name"),
            Self::InvalidEmail => Some("email"),
            Self::InvalidSource => Some("source"),
            Self::InvalidId => Some("id"),
            Self::InvalidCsv => Some("csv"),
            Self::InvalidGroups => Some("groups"),
            Self::InvalidField => Some("field"),
            Self::InvalidValue => Some("value"),
            Self::InvalidRecipientId => Some("recipient_id"),
            Self::InvalidGroupId => Some("groups"),
            Self::InvalidSubject => Some("subject"),
            Self::InvalidCooldownDays => Some("cooldown_days"),
            Self::InvalidAttachment => Some("attachment"),
            Self::InvalidRecipients => Some("recipients"),
            Self::InvalidImage => Some("image"),
        }
    }
}

fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, field_errors)| {
            field_errors.iter().map(|error| FormFieldError {
                field: field.clone(),
                message: validation_error_message(error),
            })
        })
        .collect()
}

fn validation_error_message(error: &ValidationError) -> Cow<'static, str> {
    error
        .message
        .clone()
        .unwrap_or(Cow::Borrowed("Поле заполнено некорректно."))
}

fn validation_errors_display(errors: &ValidationErrors) -> String {
    let messages = collect_validation_errors(errors)
        .into_iter()
        .map(|error| error.message.into_owned())
        .collect::<Vec<_>>();

    if messages.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else {
        format!("Ошибка валидации формы: {}", messages.join("; "))
    }
}

#[allow(dead_code)]
fn field_error(field: &'static str, message: impl Into<Cow<'static, str>>) -> FormFieldError {
    FormFieldError {
        field: Cow::Borrowed(field),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::FormError;
    use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm};
    use crate::forms::recipients::{AddRecipientForm, SourceRecipientForm};
    use validator::Validate;

    fn field_errors(error: &FormError) -> Vec<(String, String)> {
        let mut field_errors = error
            .field_errors()
            .into_iter()
            .map(|error| (error.field.to_string(), error.message.into_owned()))
            .collect::<Vec<_>>();
        field_errors.sort();
        field_errors
    }

    #[test]
    fn validation_errors_use_messages_declared_by_forms() {
        let error = FormError::from(
            AddRecipientForm {
                name: String::new(),
                email: "invalid".to_string(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&error),
            vec![
                (
                    "email".to_string(),
                    "Укажите корректный электронный адрес.".to_string(),
                ),
                ("name".to_string(), "Укажите имя.".to_string()),
            ]
        );
    }

    #[test]
    fn url_and_recipient_selection_messages_come_from_forms() {
        let source_error = FormError::from(
            SourceRecipientForm {
                source: "invalid-url".to_string(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );
        let assign_error = FormError::from(
            AssignGroupRecipientForm {
                recipient_id: Vec::new(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            field_errors(&source_error),
            vec![("source".to_string(), "Укажите корректный URL.".to_string(),)]
        );
        assert_eq!(
            field_errors(&assign_error),
            vec![(
                "recipient_id".to_string(),
                "Выберите получателей.".to_string(),
            )]
        );
    }

    #[test]
    fn conversion_error_messages_stay_in_forms_layer() {
        assert_eq!(
            field_errors(&FormError::InvalidGroupId),
            vec![(
                "groups".to_string(),
                "Группа заполнена некорректно.".to_string(),
            )]
        );
        assert_eq!(FormError::InvalidName.to_string(), "Укажите имя.");

        let validation_error = FormError::from(
            AddGroupForm {
                name: String::new(),
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        assert_eq!(
            validation_error.to_string(),
            "Ошибка валидации формы: Укажите название группы."
        );
    }
}
