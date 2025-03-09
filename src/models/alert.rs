use actix_session::Session;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub category: String,
    pub message: String,
}

pub fn add_flash_message(session: &mut Session, category: &str, message: &str) {
    let mut messages: Vec<Alert> = match session.get("flash_messages") {
        Ok(Some(msgs)) => msgs, // If messages exist, use them
        Ok(None) => vec![],     // If no messages exist, initialize an empty Vec
        Err(_) => vec![],       // If an error occurs, fall back to an empty Vec
    };

    messages.push(Alert {
        category: category.to_string(),
        message: message.to_string(),
    });

    let _ = session.insert("flash_messages", messages); // Store updated messages
}

pub fn get_flash_messages(session: &mut Session) -> Vec<Alert> {
    let messages: Vec<Alert> = match session.get("flash_messages") {
        Ok(Some(msgs)) => msgs, // If messages exist, use them
        Ok(None) => vec![],     // If no messages exist, initialize an empty Vec
        Err(_) => vec![],       // If an error occurs, fall back to an empty Vec
    };
    let _ = session.remove("flash_messages"); // Clear messages after retrieval
    messages
}
