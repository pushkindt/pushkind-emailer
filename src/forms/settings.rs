use crate::models::hub::Hub;
use serde::Deserialize;

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
    pub server: Option<String>,
    pub port: Option<i32>,
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
            server: val.server,
            port: val.port,
            created_at: val.created_at,
            updated_at: Some(chrono::Utc::now().naive_utc()),
        }
    }
}

#[derive(Deserialize)]
pub struct DeleteHubForm {
    pub id: i32,
}
