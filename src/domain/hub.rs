use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
pub struct Hub {
    pub id: i32,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub email_template: Option<String>,
}

pub struct NewHub<'a> {
    pub id: i32,
    pub login: Option<&'a str>,
    pub password: Option<&'a str>,
    pub sender: Option<&'a str>,
    pub smtp_server: Option<&'a str>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<&'a str>,
    pub imap_port: Option<i32>,
    pub email_template: Option<&'a str>,
}

pub struct UpdateHub<'a> {
    pub login: Option<&'a str>,
    pub password: Option<&'a str>,
    pub sender: Option<&'a str>,
    pub smtp_server: Option<&'a str>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<&'a str>,
    pub imap_port: Option<i32>,
    pub email_template: Option<&'a str>,
}

impl Hub {
    pub fn get_usubscribe_url(&self) -> String {
        match &self.login {
            Some(login) => format!("mailto:{login}?subject=unsubscribe"),
            None => String::from(""),
        }
    }
}

impl<'a> NewHub<'a> {
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
