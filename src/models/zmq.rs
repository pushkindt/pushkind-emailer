use serde::{Deserialize, Serialize};

use pushkind_common::domain::auth::AuthenticatedUser;

use crate::domain::email::NewEmail;

#[derive(Serialize, Deserialize)]
pub enum ZMQSendEmailMessage {
    NewEmail(Box<(AuthenticatedUser, NewEmail)>),
    RetryEmail((i32, i32)), // (id, hub_id)
}

#[derive(Serialize, Deserialize)]
pub struct ZMQReplyMessage {
    pub hub_id: i32,
    pub email: String,
    pub message: String,
    pub subject: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ZMQUnsubscribeMessage {
    pub hub_id: i32,
    pub email: String,
    pub reason: Option<String>,
}
