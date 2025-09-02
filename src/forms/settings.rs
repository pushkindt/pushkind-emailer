use pushkind_common::domain::emailer::hub::UpdateHub;
use serde::Deserialize;

/// Form to create a new hub configuration.
#[derive(Deserialize)]
pub struct AddHubForm {
    pub hub_name: String,
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

impl From<SaveHubForm> for UpdateHub {
    fn from(val: SaveHubForm) -> Self {
        Self {
            login: val.login,
            password: val.password,
            sender: val.sender,
            smtp_server: val.smtp_server,
            smtp_port: val.smtp_port,
            imap_server: val.imap_server,
            imap_port: val.imap_port,
            created_at: val.created_at,
            updated_at: Some(chrono::Utc::now().naive_utc()),
            email_template: val.message,
        }
    }
}

/// Form to remove a hub from the system.
#[derive(Deserialize)]
pub struct DeleteHubForm {
    pub id: i32,
}
