use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ZMQMessage {
    pub email_id: i32,
}
