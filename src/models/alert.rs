use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Alert {
    pub category: String,
    pub message: String,
}
