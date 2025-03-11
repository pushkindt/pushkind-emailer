use serde::Deserialize;

#[derive(Deserialize)]
pub struct SendEmailForm {
    #[serde(default)]
    pub recipients: Vec<String>,
    pub message: String,
    pub subject: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteEmailForm {
    pub id: i32,
}
