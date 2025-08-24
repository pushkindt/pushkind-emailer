use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
/// An email message stored in the system.
pub struct Email {
    /// Database identifier of the email.
    pub id: i32,
    /// Raw body of the message that will be sent to recipients.
    pub message: String,
    /// Time the email record was created.
    pub created_at: NaiveDateTime,
    /// Whether the email has already been sent.
    pub is_sent: bool,
    /// Optional subject line for the message.
    pub subject: Option<String>,
    /// Optional binary attachment.
    pub attachment: Option<Vec<u8>>,
    /// File name of the attachment, if any.
    pub attachment_name: Option<String>,
    /// MIME type of the attachment.
    pub attachment_mime: Option<String>,
    /// Number of recipients the email was sent to.
    pub num_sent: i32,
    /// Number of recipients that opened the email.
    pub num_opened: i32,
    /// Number of recipients that replied to the email.
    pub num_replied: i32,
    /// Hub that owns this email.
    pub hub_id: i32,
}

#[derive(Serialize)]
/// A single email address targeted by an email.
pub struct EmailRecipient {
    /// Identifier of the record.
    pub id: i32,
    /// Associated [`Email`] id.
    pub email_id: i32,
    /// Recipient email address.
    pub address: String,
    /// Whether the message was opened by the recipient.
    pub opened: bool,
    /// Last time the recipient record was updated.
    pub updated_at: NaiveDateTime,
    /// Flag indicating the email was sent to this recipient.
    pub is_sent: bool,
    /// Whether the recipient replied.
    pub replied: bool,
    /// Optional recipient name at the moment of sending
    pub name: Option<String>,
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
    pub address: String,
    /// Optional recipient name.
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
/// Parameters required to create a new [`Email`].
pub struct NewEmail {
    /// Body of the message to be sent.
    pub message: String,
    /// Optional subject line.
    pub subject: Option<String>,
    /// Optional binary attachment for the email.
    pub attachment: Option<Vec<u8>>,
    /// Name of the attachment file.
    pub attachment_name: Option<String>,
    /// MIME type of the attachment.
    pub attachment_mime: Option<String>,
    /// Hub to which the email belongs.
    pub hub_id: i32,
    /// List of recipient email addresses.
    pub recipients: Vec<NewEmailRecipient>,
}

/// Counters used to update email statistics.
pub struct UpdateEmail {
    /// Total number of times the email was sent.
    pub num_sent: i32,
    /// How many recipients opened the email.
    pub num_opened: i32,
    /// How many recipients replied to the email.
    pub num_replied: i32,
}

/// Changes to apply to an [`EmailRecipient`] record.
pub struct UpdateEmailRecipient {
    /// Updated open status.
    pub opened: Option<bool>,
    /// Updated sent status.
    pub is_sent: Option<bool>,
    /// Updated reply status.
    pub replied: Option<bool>,
}
