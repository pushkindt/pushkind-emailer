use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
/// Configuration and metadata for an email hub.
pub struct Hub {
    /// Identifier of the hub.
    pub id: i32,
    /// Optional login used for sending emails.
    pub login: Option<String>,
    /// Optional password for the login.
    pub password: Option<String>,
    /// Sender address that appears in outgoing emails.
    pub sender: Option<String>,
    /// SMTP server hostname.
    pub smtp_server: Option<String>,
    /// SMTP server port.
    pub smtp_port: Option<i32>,
    /// When the hub was created.
    pub created_at: Option<NaiveDateTime>,
    /// When the hub settings were last updated.
    pub updated_at: Option<NaiveDateTime>,
    /// IMAP server hostname for reading replies.
    pub imap_server: Option<String>,
    /// IMAP server port.
    pub imap_port: Option<i32>,
    /// Template applied to outgoing emails.
    pub email_template: Option<String>,
    /// Last IMAP message ID seen by the hub.
    pub imap_last_uid: i32,
}

/// Data required to create a new [`Hub`].
pub struct NewHub {
    /// Identifier of the hub to be created.
    pub id: i32,
    /// Login used for SMTP authentication.
    pub login: Option<String>,
    /// Password for the SMTP login.
    pub password: Option<String>,
    /// Sender address used in outgoing emails.
    pub sender: Option<String>,
    /// SMTP server hostname.
    pub smtp_server: Option<String>,
    /// SMTP server port.
    pub smtp_port: Option<i32>,
    /// Creation timestamp.
    pub created_at: Option<NaiveDateTime>,
    /// Last update timestamp.
    pub updated_at: Option<NaiveDateTime>,
    /// IMAP server hostname.
    pub imap_server: Option<String>,
    /// IMAP server port.
    pub imap_port: Option<i32>,
    /// Template applied to outgoing emails.
    pub email_template: Option<String>,
}

/// Fields that can be updated for an existing [`Hub`].
pub struct UpdateHub {
    /// New login for SMTP authentication.
    pub login: Option<String>,
    /// New password for the login.
    pub password: Option<String>,
    /// Updated sender address.
    pub sender: Option<String>,
    /// Updated SMTP server hostname.
    pub smtp_server: Option<String>,
    /// Updated SMTP port.
    pub smtp_port: Option<i32>,
    /// Updated creation timestamp.
    pub created_at: Option<NaiveDateTime>,
    /// Updated modification timestamp.
    pub updated_at: Option<NaiveDateTime>,
    /// Updated IMAP server hostname.
    pub imap_server: Option<String>,
    /// Updated IMAP port.
    pub imap_port: Option<i32>,
    /// Updated email template.
    pub email_template: Option<String>,
}

impl Hub {
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
    pub fn new(id: i32) -> Self {
        Self {
            id,
            login: None,
            password: None,
            sender: None,
            smtp_server: None,
            smtp_port: None,
            created_at: None,
            updated_at: None,
            imap_server: None,
            imap_port: None,
            email_template: None,
        }
    }
}
