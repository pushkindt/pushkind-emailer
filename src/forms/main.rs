use serde::Deserialize;

#[derive(Deserialize)]
pub struct SendEmailForm {
    #[serde(default)]
    pub recipients: Vec<String>,
    pub message: String,
}

#[derive(Deserialize)]
pub struct DeleteEmailForm {
    pub id: i32,
}
