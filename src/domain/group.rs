use chrono::NaiveDateTime;
use serde::Serialize;

use crate::domain::recipient::Recipient;

#[derive(Serialize)]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub hub_id: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Serialize)]
pub struct GroupWithRecipients {
    pub group: Group,
    pub recipients: Vec<Recipient>,
}

pub struct NewGroup<'a> {
    pub name: &'a str,
    pub hub_id: i32,
}
