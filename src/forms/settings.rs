use serde::Deserialize;

use crate::models::hub::Hub;

#[derive(Deserialize)]
pub struct AddHubForm {
    pub hub_name: String,
}

#[derive(Deserialize)]
pub struct ActivateHubForm {
    pub hub_id: i32,
}

#[derive(Deserialize)]
pub struct SaveHubForm {
    pub id: i32,
    pub name: String,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

impl From<SaveHubForm> for Hub {
    fn from(val: SaveHubForm) -> Self {
        Hub {
            id: val.id,
            name: val.name,
            login: val.login,
            password: val.password,
            sender: val.sender,
            smtp_server: val.smtp_server,
            smtp_port: val.smtp_port,
            imap_server: val.imap_server,
            imap_port: val.imap_port,
            created_at: val.created_at,
            updated_at: Some(chrono::Utc::now().naive_utc()),
        }
    }
}

#[derive(Deserialize)]
pub struct DeleteHubForm {
    pub id: i32,
}
