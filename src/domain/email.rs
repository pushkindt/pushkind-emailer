use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    AttachmentMimeType, AttachmentName, EmailBody, EmailId, EmailOpenedCount, EmailRecipientId,
    EmailRecipientReply, EmailRepliedCount, EmailSentCount, EmailSubject, HubId, RecipientEmail,
    RecipientName, TypeConstraintError,
};

#[derive(Serialize)]
/// An email message stored in the system.
pub struct Email {
    /// Database identifier of the email.
    pub id: EmailId,
    /// Raw body of the message that will be sent to recipients.
    pub message: EmailBody,
    /// Time the email record was created.
    pub created_at: NaiveDateTime,
    /// Whether the email has already been sent.
    pub is_sent: bool,
    /// Optional subject line for the message.
    pub subject: Option<EmailSubject>,
    /// Optional binary attachment.
    pub attachment: Option<Vec<u8>>,
    /// File name of the attachment, if any.
    pub attachment_name: Option<AttachmentName>,
    /// MIME type of the attachment.
    pub attachment_mime: Option<AttachmentMimeType>,
    /// Number of recipients the email was sent to.
    pub num_sent: EmailSentCount,
    /// Number of recipients that opened the email.
    pub num_opened: EmailOpenedCount,
    /// Number of recipients that replied to the email.
    pub num_replied: EmailRepliedCount,
    /// Hub that owns this email.
    pub hub_id: HubId,
}

impl Email {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        message: impl Into<String>,
        created_at: NaiveDateTime,
        is_sent: bool,
        subject: Option<String>,
        attachment: Option<Vec<u8>>,
        attachment_name: Option<String>,
        attachment_mime: Option<String>,
        num_sent: i32,
        num_opened: i32,
        num_replied: i32,
        hub_id: i32,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            id: EmailId::try_from(id)?,
            message: EmailBody::new(message.into())?,
            created_at,
            is_sent,
            subject: subject.map(EmailSubject::try_from).transpose()?,
            attachment,
            attachment_name: attachment_name
                .filter(|s| !s.trim().is_empty())
                .map(AttachmentName::try_from)
                .transpose()?,
            attachment_mime: attachment_mime
                .filter(|s| !s.trim().is_empty())
                .map(AttachmentMimeType::try_from)
                .transpose()?,
            num_sent: EmailSentCount::try_from(num_sent)?,
            num_opened: EmailOpenedCount::try_from(num_opened)?,
            num_replied: EmailRepliedCount::try_from(num_replied)?,
            hub_id: HubId::try_from(hub_id)?,
        })
    }
}

#[derive(Serialize)]
/// A single email address targeted by an email.
pub struct EmailRecipient {
    /// Identifier of the record.
    pub id: EmailRecipientId,
    /// Associated [`Email`] id.
    pub email_id: EmailId,
    /// Recipient email address.
    pub address: RecipientEmail,
    /// Whether the message was opened by the recipient.
    pub opened: bool,
    /// Last time the recipient record was updated.
    pub updated_at: NaiveDateTime,
    /// Flag indicating the email was sent to this recipient.
    pub is_sent: bool,
    /// Whether the recipient replied.
    pub replied: bool,
    /// Optional recipient's reply
    pub reply: Option<EmailRecipientReply>,
    /// Recipient's name at the moment of sending
    pub name: RecipientName,
    /// Recipient's JSON-encoded fields at the moment of sending
    pub fields: HashMap<String, String>,
}

impl EmailRecipient {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        email_id: i32,
        address: impl Into<String>,
        opened: bool,
        updated_at: NaiveDateTime,
        is_sent: bool,
        replied: bool,
        reply: Option<String>,
        name: impl Into<String>,
        fields: HashMap<String, String>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            id: EmailRecipientId::try_from(id)?,
            email_id: EmailId::try_from(email_id)?,
            address: RecipientEmail::new(address.into())?,
            opened,
            updated_at,
            is_sent,
            replied,
            reply: reply
                .filter(|s| !s.trim().is_empty())
                .map(EmailRecipientReply::try_from)
                .transpose()?,
            name: RecipientName::new(name.into())?,
            fields,
        })
    }
}

#[derive(Serialize)]
/// A convenience wrapper containing an email and its recipients.
pub struct EmailWithRecipients {
    /// The email message.
    pub email: Email,
    /// Recipients of the email.
    pub recipients: Vec<EmailRecipient>,
}

#[derive(Serialize, Deserialize)]
pub struct NewEmailRecipient {
    /// Email address of the recipient.
    pub address: RecipientEmail,
    /// Recipient's name
    pub name: RecipientName,
    /// Optional recipient fields.
    pub fields: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
/// Parameters required to create a new [`Email`].
pub struct NewEmail {
    /// Body of the message to be sent.
    pub message: EmailBody,
    /// Optional subject line.
    pub subject: Option<EmailSubject>,
    /// Optional binary attachment for the email.
    pub attachment: Option<Vec<u8>>,
    /// Name of the attachment file.
    pub attachment_name: Option<AttachmentName>,
    /// MIME type of the attachment.
    pub attachment_mime: Option<AttachmentMimeType>,
    /// Hub to which the email belongs.
    pub hub_id: HubId,
    /// List of recipient email addresses.
    pub recipients: Vec<NewEmailRecipient>,
}

impl NewEmail {
    pub fn try_new(
        hub_id: i32,
        message: impl Into<String>,
        subject: Option<String>,
        attachment: Option<Vec<u8>>,
        attachment_name: Option<String>,
        attachment_mime: Option<String>,
        recipients: Vec<NewEmailRecipient>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            hub_id: HubId::try_from(hub_id)?,
            message: EmailBody::new(message.into())?,
            subject: subject
                .and_then(|s| {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .map(EmailSubject::try_from)
                .transpose()?,
            attachment,
            attachment_name: attachment_name
                .and_then(|s| {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .map(AttachmentName::try_from)
                .transpose()?,
            attachment_mime: attachment_mime
                .and_then(|s| {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .map(AttachmentMimeType::try_from)
                .transpose()?,
            recipients,
        })
    }
}

/// Counters used to update email statistics.
pub struct UpdateEmail {
    /// Total number of times the email was sent.
    pub num_sent: EmailSentCount,
    /// How many recipients opened the email.
    pub num_opened: EmailOpenedCount,
    /// How many recipients replied to the email.
    pub num_replied: EmailRepliedCount,
}

/// Changes to apply to an [`EmailRecipient`] record.
pub struct UpdateEmailRecipient {
    /// Updated open status.
    pub opened: Option<bool>,
    /// Updated sent status.
    pub is_sent: Option<bool>,
    /// Updated reply status.
    pub replied: Option<bool>,
    /// Optional recipient's reply
    pub reply: Option<EmailRecipientReply>,
}
