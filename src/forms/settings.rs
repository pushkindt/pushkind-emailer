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
}

impl Into<Hub> for SaveHubForm {
    fn into(self) -> Hub {
        Hub {
            id: self.id,
            name: self.name,
            login: self.login,
            password: self.password,
            sender: self.sender,
            server: self.server,
            port: self.port,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Deserialize)]
pub struct DeleteHubForm {
    pub id: i32,
}
