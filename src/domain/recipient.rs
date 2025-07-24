use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::domain::group::Group;

#[derive(Serialize)]
pub struct Recipient {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub hub_id: i32,
    pub fields: HashMap<String, String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub unsubscribed_at: Option<NaiveDateTime>,
    pub groups: Vec<i32>,
}

#[derive(Serialize)]
pub struct RecipientWithGroups {
    pub recipient: Recipient,
    pub groups: Vec<Group>,
}

pub struct NewRecipient<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub hub_id: i32,
}

pub struct UpdateRecipient {
    pub name: String,
    pub email: String,
    pub fields: HashMap<String, String>,
    pub unsubscribed_at: Option<NaiveDateTime>,
    pub groups: Vec<i32>,
}
