use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddRecipientForm {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct DeleteRecipientForm {
    pub id: i32,
}
