use serde::{Deserialize, Serialize};

use crate::domain::email::NewEmail;

#[derive(Serialize, Deserialize)]
pub enum ZMQSendEmailMessage {
    NewEmail(NewEmail),
    RetryEmail(i32),
}
