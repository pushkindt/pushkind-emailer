use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize, AsChangeset, Deserialize)]
#[diesel(table_name = crate::schema::hubs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Hub {
    pub id: i32,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub email_template: Option<String>,
}

impl Hub {
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
    pub fn get_usubscribe_url(&self) -> String {
        match &self.login {
            Some(login) => format!("mailto:{}?subject=unsubscribe", login),
            None => String::from(""),
        }
    }
}
