use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddGroupForm {
    pub name: String,
}

#[derive(Deserialize)]
pub struct DeleteGroupForm {
    pub id: i32,
}

#[derive(Deserialize)]
pub struct AssignGroupRecipientForm {
    pub recipient_id: i32,
    pub group_id: i32,
}
