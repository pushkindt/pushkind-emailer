//! Web form types used by the application.
//!
//! Each submodule contains structs representing data submitted by the user via
//! HTTP forms or multipart requests.
use thiserror::Error;
use validator::ValidationErrors;

pub mod files;
pub mod groups;
pub mod main;
pub mod recipients;
pub mod settings;

/// Enumerates all possible form field validation errors.
#[derive(Debug, Error)]
pub enum FormError {
    #[error("validation errors: {0}")]
    Validation(#[from] ValidationErrors),
    /// Invalid `login` field.
    #[error("invalid login")]
    InvalidLogin,
    /// Invalid `password` field.
    #[error("invalid password")]
    InvalidPassword,
    /// Invalid `sender` field.
    #[error("invalid sender")]
    InvalidSender,
    /// Invalid `smtp_server` field.
    #[error("invalid smtp server")]
    InvalidSmtpServer,
    /// Invalid `smtp_port` field.
    #[error("invalid smtp port")]
    InvalidSmtpPort,
    /// Invalid `imap_server` field.
    #[error("invalid imap server")]
    InvalidImapServer,
    /// Invalid `imap_port` field.
    #[error("invalid imap port")]
    InvalidImapPort,
    /// Invalid `created_at` field.
    #[error("invalid created at")]
    InvalidCreatedAt,
    /// Invalid `message` field.
    #[error("invalid message")]
    InvalidMessage,
    /// Invalid `name` field.
    #[error("invalid name")]
    InvalidName,
    /// Invalid `email` field.
    #[error("invalid email")]
    InvalidEmail,
    /// Invalid `source` field.
    #[error("invalid source")]
    InvalidSource,
    /// Invalid `id` field.
    #[error("invalid id")]
    InvalidId,
    /// Invalid `csv` field.
    #[error("invalid csv")]
    InvalidCsv,
    /// Invalid `groups` field.
    #[error("invalid groups")]
    InvalidGroups,
    /// Invalid `field` field.
    #[error("invalid field")]
    InvalidField,
    /// Invalid `value` field.
    #[error("invalid value")]
    InvalidValue,
    /// Invalid `recipient_id` field.
    #[error("invalid recipient id")]
    InvalidRecipientId,
    /// Invalid `group_id` field.
    #[error("invalid group id")]
    InvalidGroupId,
    /// Invalid `subject` field.
    #[error("invalid subject")]
    InvalidSubject,
    /// Invalid `cooldown_days` field.
    #[error("invalid cooldown days")]
    InvalidCooldownDays,
    /// Invalid `attachment` field.
    #[error("invalid attachment")]
    InvalidAttachment,
    /// Invalid `recipients` field.
    #[error("invalid recipients")]
    InvalidRecipients,
    /// Invalid `image` field.
    #[error("invalid image")]
    InvalidImage,
}
