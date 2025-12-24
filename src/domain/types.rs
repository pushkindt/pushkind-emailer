//! Strongly-typed value objects used by domain entities.
//!
//! These wrappers enforce basic invariants (e.g., positive identifiers,
//! normalized/validated email) so that once a value reaches the domain layer it
//! can be treated as trusted.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use validator::{ValidateEmail, ValidateUrl};

/// Errors produced when attempting to construct a constrained value object.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TypeConstraintError {
    /// Provided identifier is zero or negative.
    #[error("id must be greater than zero")]
    NonPositiveId,
    /// Provided value is negative where only non-negative values are allowed.
    #[error("value must be zero or greater")]
    NegativeValue,
    /// Provided email failed format validation.
    #[error("invalid email address")]
    InvalidEmail,
    /// Provided string contained no non-whitespace characters.
    #[error("value cannot be empty")]
    EmptyString,
    /// Provided port number is out of the valid TCP/UDP range.
    #[error("port must be between 1 and 65535")]
    InvalidPort,
    /// Provided value failed custom validation.
    #[error("invalid value: {0}")]
    InvalidValue(String),
    /// Provided url failed format validation.
    #[error("invalid url address")]
    InvalidUrl,
}

/// Normalizes and validates an email string.
fn normalize_email<S: Into<String>>(email: S) -> Result<String, TypeConstraintError> {
    let normalized = email.into().trim().to_lowercase();
    if normalized.validate_email() {
        Ok(normalized)
    } else {
        Err(TypeConstraintError::InvalidEmail)
    }
}

/// Macro to generate lightweight newtypes for positive identifiers.
macro_rules! id_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(i32);

        impl $name {
            /// Creates a new identifier ensuring it is greater than zero.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value > 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NonPositiveId)
                }
            }

            /// Returns the raw `i32` backing this identifier.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

id_newtype!(RecipientId, "Unique identifier for a recipient.");
id_newtype!(HubId, "Unique identifier for a hub.");
id_newtype!(GroupId, "Unique identifier for a group.");
id_newtype!(EmailId, "Unique identifier for an email.");
id_newtype!(
    EmailRecipientId,
    "Unique identifier for an email recipient snapshot."
);

/// Lower-cased and validated email address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RecipientEmail(String);

impl RecipientEmail {
    /// Validates and normalizes an email string.
    pub fn new<S: Into<String>>(email: S) -> Result<Self, TypeConstraintError> {
        let normalized = normalize_email(email)?;
        Ok(Self(normalized))
    }

    /// Borrow the email as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the owned inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for RecipientEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for RecipientEmail {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for RecipientEmail {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RecipientEmail> for String {
    fn from(value: RecipientEmail) -> Self {
        value.0
    }
}

macro_rules! email_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            /// Validates and normalizes an email string.
            pub fn new<S: Into<String>>(email: S) -> Result<Self, TypeConstraintError> {
                Ok(Self(normalize_email(email)?))
            }

            /// Borrow the email as a `&str`.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Convert into the owned inner `String`.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

email_newtype!(
    HubLogin,
    "SMTP/IMAP login; normalized and validated as an email address."
);

/// Wrapper for non-empty, trimmed strings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Trims whitespace and rejects empty inputs.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(TypeConstraintError::EmptyString);
        }
        Ok(Self(trimmed))
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper returning the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

macro_rules! non_empty_string_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Constructs a trimmed, non-empty value.
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let inner = NonEmptyString::new(value)?;
                Ok(Self(inner.into_inner()))
            }

            /// Borrow the value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the wrapper and return the owned string.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_empty_string_newtype!(
    RecipientName,
    "Recipient user name wrapper enforcing non-empty values."
);

non_empty_string_newtype!(GroupName, "Group name wrapper enforcing non-empty values.");

non_empty_string_newtype!(EmailBody, "Email body wrapper enforcing non-empty values.");
non_empty_string_newtype!(
    EmailSubject,
    "Email subject wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    EmailRecipientReply,
    "Recipient reply message wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    AttachmentName,
    "Attachment filename wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    AttachmentMimeType,
    "Attachment MIME type wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    SmtpServerHost,
    "SMTP server host wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    ImapServerHost,
    "IMAP server host wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    UnsubscribeReason,
    "Unsubscribe reason wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    CustomFieldKey,
    "Custom field key wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    CustomFieldValue,
    "Custom field value wrapper enforcing non-empty values."
);
non_empty_string_newtype!(
    HubSenderName,
    "Sender name that appears in outgoing emails; normalized and validated."
);

/// A secret string used for hub SMTP/IMAP authentication.
///
/// This type intentionally redacts its value in `Debug` output to reduce the
/// risk of leaking secrets into logs.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HubPassword(String);

impl HubPassword {
    /// Trims whitespace and rejects empty inputs.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let inner = NonEmptyString::new(value)?.into_inner();
        Ok(Self(inner))
    }

    /// Borrow the inner password string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper returning the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for HubPassword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HubPassword").field(&"<redacted>").finish()
    }
}

impl TryFrom<String> for HubPassword {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for HubPassword {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<HubPassword> for String {
    fn from(value: HubPassword) -> Self {
        value.0
    }
}

/// Email message template enforcing trimmed, non-empty values.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmailTemplate(String);

impl EmailTemplate {
    /// Constructs a sanitized, trimmed, non-empty value.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let sanitized = ammonia::clean(&value.into());
        let inner = NonEmptyString::new(sanitized)?;
        Ok(Self(inner.into_inner()))
    }

    /// Borrow the value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for EmailTemplate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for EmailTemplate {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for EmailTemplate {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<EmailTemplate> for String {
    fn from(value: EmailTemplate) -> Self {
        value.0
    }
}

macro_rules! port_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(u16);

        impl $name {
            /// Creates a new port ensuring it is in the range `1..=65535`.
            pub fn new(value: u16) -> Result<Self, TypeConstraintError> {
                if value == 0 {
                    Err(TypeConstraintError::InvalidPort)
                } else {
                    Ok(Self(value))
                }
            }

            /// Returns the port as `u16`.
            pub const fn get(self) -> u16 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                let Ok(value) = u16::try_from(value) else {
                    return Err(TypeConstraintError::InvalidPort);
                };
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                i32::from(value.0)
            }
        }
    };
}

port_newtype!(SmtpPort, "SMTP server port.");
port_newtype!(ImapPort, "IMAP server port.");

macro_rules! non_negative_i32_newtype {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(i32);

        impl $name {
            /// Creates a new value ensuring it is non-negative.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value >= 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NegativeValue)
                }
            }

            /// Returns the raw `i32` backing this value.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_negative_i32_newtype!(ImapUid, "IMAP UID marker; non-negative.");
non_negative_i32_newtype!(EmailSentCount, "Number of recipients an email was sent to.");
non_negative_i32_newtype!(
    EmailOpenedCount,
    "Number of recipients that opened an email."
);
non_negative_i32_newtype!(
    EmailRepliedCount,
    "Number of recipients that replied to an email."
);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Non-empty, trimmed menu URL.
pub struct RecipientSourceUrl(String);

impl RecipientSourceUrl {
    /// Ensures a trimmed menu URL is non-empty before wrapping.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        let url = NonEmptyString::new(value)?;

        if !url.as_str().validate_url() {
            Err(TypeConstraintError::InvalidUrl)
        } else {
            Ok(Self(url.into_inner()))
        }
    }

    /// Borrow the menu URL.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the owned menu URL.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for RecipientSourceUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for RecipientSourceUrl {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for RecipientSourceUrl {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<RecipientSourceUrl> for String {
    fn from(value: RecipientSourceUrl) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_password_redacts_debug() {
        let password = HubPassword::new("secret").unwrap();
        let debug = format!("{password:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn smtp_port_rejects_zero() {
        assert_eq!(
            SmtpPort::new(0).unwrap_err(),
            TypeConstraintError::InvalidPort
        );
    }

    #[test]
    fn imap_uid_rejects_negative() {
        assert_eq!(
            ImapUid::new(-1).unwrap_err(),
            TypeConstraintError::NegativeValue
        );
    }
}
