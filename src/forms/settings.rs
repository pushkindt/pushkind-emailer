//! Settings-related form types and input validation.
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::hub::UpdateHub;
use crate::domain::types::{
    EmailTemplate, HubLogin, HubPassword, HubSenderName, ImapPort, ImapServerHost, SmtpPort,
    SmtpServerHost,
};
use crate::forms::FormError;

/// Form for updating hub configuration details.
#[derive(Deserialize, Validate)]
pub struct SaveHubForm {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub login: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub password: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub sender: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub smtp_server: Option<String>,
    #[validate(range(min = 0, message = "Укажите корректный SMTP порт."))]
    pub smtp_port: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub imap_server: Option<String>,
    #[validate(range(min = 0, message = "Укажите корректный IMAP порт."))]
    pub imap_port: Option<i32>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
}

pub struct SaveHubPayload {
    pub login: Option<HubLogin>,
    pub password: Option<HubPassword>,
    pub sender: Option<HubSenderName>,
    pub smtp_server: Option<SmtpServerHost>,
    pub smtp_port: Option<SmtpPort>,
    pub imap_server: Option<ImapServerHost>,
    pub imap_port: Option<ImapPort>,
    pub email_template: Option<EmailTemplate>,
}

impl TryFrom<SaveHubForm> for SaveHubPayload {
    type Error = FormError;

    fn try_from(form: SaveHubForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            login: form
                .login
                .map(HubLogin::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidLogin)?,
            password: form
                .password
                .map(HubPassword::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidPassword)?,
            sender: form
                .sender
                .map(HubSenderName::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidSender)?,
            smtp_server: form
                .smtp_server
                .map(SmtpServerHost::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidSmtpServer)?,
            smtp_port: form
                .smtp_port
                .filter(|port| *port != 0)
                .map(SmtpPort::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidSmtpPort)?,
            imap_server: form
                .imap_server
                .map(ImapServerHost::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidImapServer)?,
            imap_port: form
                .imap_port
                .filter(|port| *port != 0)
                .map(ImapPort::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidImapPort)?,
            email_template: form
                .message
                .map(EmailTemplate::try_from)
                .transpose()
                .map_err(|_| FormError::InvalidMessage)?,
        })
    }
}

impl SaveHubPayload {
    pub fn into_domain(self) -> UpdateHub {
        UpdateHub::new(
            self.login,
            self.password,
            self.sender,
            self.smtp_server,
            self.smtp_port,
            self.imap_server,
            self.imap_port,
            self.email_template,
        )
    }
}
