use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct AddGroupForm {
    #[validate(length(min = 1))]
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
