use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
pub struct Email {
    pub id: i32,
    pub message: String,
    pub created_at: NaiveDateTime,
    pub is_sent: bool,
    pub subject: Option<String>,
    pub attachment: Option<Vec<u8>>,
    pub attachment_name: Option<String>,
    pub attachment_mime: Option<String>,
    pub num_sent: i32,
    pub num_opened: i32,
    pub num_replied: i32,
    pub hub_id: i32,
}

#[derive(Serialize)]
pub struct EmailRecipient {
    pub id: i32,
    pub email_id: i32,
    pub address: String,
    pub opened: bool,
    pub updated_at: NaiveDateTime,
    pub is_sent: bool,
    pub replied: bool,
}

#[derive(Serialize)]
pub struct EmailWithRecipients {
    pub email: Email,
    pub recipients: Vec<EmailRecipient>,
}

pub struct NewEmail<'a> {
    pub message: &'a str,
    pub subject: Option<&'a str>,
    pub attachment: Option<&'a [u8]>,
    pub attachment_name: Option<&'a str>,
    pub attachment_mime: Option<&'a str>,
    pub hub_id: i32,
    pub recipients: Vec<&'a str>,
}

pub struct UpdateEmail {
    pub num_sent: i32,
    pub num_opened: i32,
    pub num_replied: i32,
}

pub struct UpdateEmailRecipient {
    pub opened: bool,
    pub is_sent: bool,
    pub replied: bool,
}
