use serde::Deserialize;

use crate::domain::hub::UpdateHub;

/// Form to create a new hub configuration.
#[derive(Deserialize)]
pub struct AddHubForm {
    pub hub_name: String,
}

/// Form to activate an existing hub by its identifier.
#[derive(Deserialize)]
pub struct ActivateHubForm {
    pub hub_id: i32,
}

/// Form for updating hub configuration details.
#[derive(Deserialize)]
pub struct SaveHubForm {
    pub id: i32,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub message: Option<String>,
}

impl<'a> From<&'a SaveHubForm> for UpdateHub<'a> {
    fn from(val: &'a SaveHubForm) -> Self {
        Self {
            login: val.login.as_deref(),
            password: val.password.as_deref(),
            sender: val.sender.as_deref(),
            smtp_server: val.smtp_server.as_deref(),
            smtp_port: val.smtp_port,
            imap_server: val.imap_server.as_deref(),
            imap_port: val.imap_port,
            created_at: val.created_at,
            updated_at: Some(chrono::Utc::now().naive_utc()),
            email_template: val.message.as_deref(),
        }
    }
}

/// Form to remove a hub from the system.
#[derive(Deserialize)]
pub struct DeleteHubForm {
    pub id: i32,
}
