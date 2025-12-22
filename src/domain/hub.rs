//! Domain types representing hubs.
use chrono::{NaiveDateTime, Utc};
use serde::Serialize;

use crate::domain::types::{
    EmailTemplate, HubId, HubLogin, HubPassword, HubSenderName, ImapPort, ImapServerHost, ImapUid,
    SmtpPort, SmtpServerHost, TypeConstraintError,
};

#[derive(Serialize)]
/// Configuration and metadata for an email hub.
pub struct Hub {
    /// Identifier of the hub.
    pub id: HubId,
    /// Optional login used for sending emails.
    pub login: Option<HubLogin>,
    /// Optional password for the login.
    pub password: Option<HubPassword>,
    /// Sender address that appears in outgoing emails.
    pub sender: Option<HubSenderName>,
    /// SMTP server hostname.
    pub smtp_server: Option<SmtpServerHost>,
    /// SMTP server port.
    pub smtp_port: Option<SmtpPort>,
    /// When the hub was created.
    pub created_at: Option<NaiveDateTime>,
    /// When the hub settings were last updated.
    pub updated_at: Option<NaiveDateTime>,
    /// IMAP server hostname for reading replies.
    pub imap_server: Option<ImapServerHost>,
    /// IMAP server port.
    pub imap_port: Option<ImapPort>,
    /// Template applied to outgoing emails.
    pub email_template: Option<EmailTemplate>,
    /// Last IMAP message ID seen by the hub.
    pub imap_last_uid: ImapUid,
}

/// Data required to create a new [`Hub`].
pub struct NewHub {
    /// Identifier of the hub to be created.
    pub id: HubId,
    /// Login used for SMTP authentication.
    pub login: Option<HubLogin>,
    /// Password for the SMTP login.
    pub password: Option<HubPassword>,
    /// Sender address used in outgoing emails.
    pub sender: Option<HubSenderName>,
    /// SMTP server hostname.
    pub smtp_server: Option<SmtpServerHost>,
    /// SMTP server port.
    pub smtp_port: Option<SmtpPort>,
    /// Creation timestamp.
    pub created_at: Option<NaiveDateTime>,
    /// Last update timestamp.
    pub updated_at: Option<NaiveDateTime>,
    /// IMAP server hostname.
    pub imap_server: Option<ImapServerHost>,
    /// IMAP server port.
    pub imap_port: Option<ImapPort>,
    /// Template applied to outgoing emails.
    pub email_template: Option<EmailTemplate>,
}

/// Fields that can be updated for an existing [`Hub`].
pub struct UpdateHub {
    /// New login for SMTP authentication.
    pub login: Option<HubLogin>,
    /// New password for the login.
    pub password: Option<HubPassword>,
    /// Updated sender address.
    pub sender: Option<HubSenderName>,
    /// Updated SMTP server hostname.
    pub smtp_server: Option<SmtpServerHost>,
    /// Updated SMTP port.
    pub smtp_port: Option<SmtpPort>,
    /// Updated modification timestamp.
    pub updated_at: Option<NaiveDateTime>,
    /// Updated IMAP server hostname.
    pub imap_server: Option<ImapServerHost>,
    /// Updated IMAP port.
    pub imap_port: Option<ImapPort>,
    /// Updated email template.
    pub email_template: Option<EmailTemplate>,
}

impl Hub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: HubId,
        login: Option<HubLogin>,
        password: Option<HubPassword>,
        sender: Option<HubSenderName>,
        smtp_server: Option<SmtpServerHost>,
        smtp_port: Option<SmtpPort>,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
        imap_server: Option<ImapServerHost>,
        imap_port: Option<ImapPort>,
        email_template: Option<EmailTemplate>,
        imap_last_uid: ImapUid,
    ) -> Self {
        Self {
            id,
            login,
            password,
            sender,
            smtp_server,
            smtp_port,
            created_at,
            updated_at,
            imap_server,
            imap_port,
            email_template,
            imap_last_uid,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        login: Option<String>,
        password: Option<String>,
        sender: Option<String>,
        smtp_server: Option<String>,
        smtp_port: Option<i32>,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
        imap_server: Option<String>,
        imap_port: Option<i32>,
        email_template: Option<String>,
        imap_last_uid: i32,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            id: HubId::try_from(id)?,
            login: login.map(TryInto::try_into).transpose()?,
            password: password.map(TryInto::try_into).transpose()?,
            sender: sender.map(TryInto::try_into).transpose()?,
            smtp_server: smtp_server.map(TryInto::try_into).transpose()?,
            smtp_port: smtp_port.map(TryInto::try_into).transpose()?,
            created_at,
            updated_at,
            imap_server: imap_server.map(TryInto::try_into).transpose()?,
            imap_port: imap_port.map(TryInto::try_into).transpose()?,
            email_template: email_template.map(TryInto::try_into).transpose()?,
            imap_last_uid: ImapUid::try_from(imap_last_uid)?,
        })
    }

    /// Generates a `mailto:` link to unsubscribe from emails.
    ///
    /// If the hub has a login configured, the returned URL is of the form
    /// `mailto:<login>?subject=unsubscribe`. Otherwise an empty string is
    /// returned.
    pub fn unsubscribe_url(&self) -> String {
        match &self.login {
            Some(login) => format!("mailto:{login}?subject=unsubscribe"),
            None => String::from(""),
        }
    }
}

impl NewHub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: HubId,
        login: Option<HubLogin>,
        password: Option<HubPassword>,
        sender: Option<HubSenderName>,
        smtp_server: Option<SmtpServerHost>,
        smtp_port: Option<SmtpPort>,
        imap_server: Option<ImapServerHost>,
        imap_port: Option<ImapPort>,
        email_template: Option<EmailTemplate>,
    ) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            id,
            login,
            password,
            sender,
            smtp_server,
            smtp_port,
            created_at: Some(now),
            updated_at: Some(now),
            imap_server,
            imap_port,
            email_template,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        login: Option<impl Into<String>>,
        password: Option<impl Into<String>>,
        sender: Option<impl Into<String>>,
        smtp_server: Option<impl Into<String>>,
        smtp_port: Option<impl Into<u16>>,
        imap_server: Option<impl Into<String>>,
        imap_port: Option<impl Into<u16>>,
        email_template: Option<impl Into<String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::new(id)?,
            login.map(HubLogin::new).transpose()?,
            password.map(HubPassword::new).transpose()?,
            sender.map(HubSenderName::new).transpose()?,
            smtp_server.map(SmtpServerHost::new).transpose()?,
            smtp_port.map(|x| SmtpPort::new(x.into())).transpose()?,
            imap_server.map(ImapServerHost::new).transpose()?,
            imap_port.map(|x| ImapPort::new(x.into())).transpose()?,
            email_template.map(EmailTemplate::new).transpose()?,
        ))
    }
}

impl UpdateHub {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        login: Option<HubLogin>,
        password: Option<HubPassword>,
        sender: Option<HubSenderName>,
        smtp_server: Option<SmtpServerHost>,
        smtp_port: Option<SmtpPort>,
        imap_server: Option<ImapServerHost>,
        imap_port: Option<ImapPort>,
        email_template: Option<EmailTemplate>,
    ) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            login,
            password,
            sender,
            smtp_server,
            smtp_port,
            updated_at: Some(now),
            imap_server,
            imap_port,
            email_template,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        login: Option<impl Into<String>>,
        password: Option<impl Into<String>>,
        sender: Option<impl Into<String>>,
        smtp_server: Option<impl Into<String>>,
        smtp_port: Option<impl Into<u16>>,
        imap_server: Option<impl Into<String>>,
        imap_port: Option<impl Into<u16>>,
        email_template: Option<impl Into<String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            login.map(HubLogin::new).transpose()?,
            password.map(HubPassword::new).transpose()?,
            sender.map(HubSenderName::new).transpose()?,
            smtp_server.map(SmtpServerHost::new).transpose()?,
            smtp_port.map(|x| SmtpPort::new(x.into())).transpose()?,
            imap_server.map(ImapServerHost::new).transpose()?,
            imap_port.map(|x| ImapPort::new(x.into())).transpose()?,
            email_template.map(EmailTemplate::new).transpose()?,
        ))
    }
}
