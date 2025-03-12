use actix_session::Session;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub category: String,
    pub message: String,
}

fn get_session_messages(session: &Session) -> Vec<Alert> {
    session
        .get("flash_messages")
        .unwrap_or_default()
        .unwrap_or_default()
}

pub fn add_flash_message(session: &mut Session, category: &str, message: &str) {
    let mut messages = get_session_messages(session);
    messages.push(Alert {
        category: category.to_string(),
        message: message.to_string(),
    });

    let _ = session.insert("flash_messages", messages); // Store updated messages
}

pub fn get_flash_messages(session: &mut Session) -> Vec<Alert> {
    let messages = get_session_messages(session);
    let _ = session.remove("flash_messages"); // Clear messages after retrieval
    messages
}
