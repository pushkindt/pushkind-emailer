//! Settings-related form types and input validation.
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::hub::UpdateHub;
use crate::domain::types::{
    EmailTemplate, HubLogin, HubPassword, HubSenderEmail, ImapPort, ImapServerHost, SmtpPort,
    SmtpServerHost, TypeConstraintError,
};

/// Form to create a new hub configuration.
#[derive(Deserialize, Validate)]
pub struct AddHubForm {
    #[validate(length(min = 1))]
    pub hub_name: String,
}

/// Form for updating hub configuration details.
#[derive(Deserialize, Validate)]
pub struct SaveHubForm {
    #[validate(range(min = 1))]
    pub id: i32,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub login: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub password: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub sender: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub imap_server: Option<String>,
    #[validate(range(min = 0))]
    pub imap_port: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
}

impl SaveHubForm {
    pub fn try_into_update_hub(self) -> Result<UpdateHub, TypeConstraintError> {
        Ok(UpdateHub {
            login: self.login.map(HubLogin::try_from).transpose()?,
            password: self.password.map(HubPassword::try_from).transpose()?,
            sender: self.sender.map(HubSenderEmail::try_from).transpose()?,
            smtp_server: self.smtp_server.map(SmtpServerHost::try_from).transpose()?,
            smtp_port: self
                .smtp_port
                .filter(|port| *port != 0)
                .map(SmtpPort::try_from)
                .transpose()?,
            imap_server: self.imap_server.map(ImapServerHost::try_from).transpose()?,
            imap_port: self
                .imap_port
                .filter(|port| *port != 0)
                .map(ImapPort::try_from)
                .transpose()?,
            created_at: self.created_at,
            updated_at: Some(chrono::Utc::now().naive_utc()),
            email_template: self.message.map(EmailTemplate::try_from).transpose()?,
        })
    }
}

/// Form to remove a hub from the system.
#[derive(Deserialize, Validate)]
pub struct DeleteHubForm {
    #[validate(range(min = 1))]
    pub id: i32,
}
