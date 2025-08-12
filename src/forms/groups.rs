use serde::Deserialize;
use validator::Validate;

/// Form data for creating a new recipient group.
#[derive(Deserialize, Validate)]
pub struct AddGroupForm {
    #[validate(length(min = 1))]
    pub name: String,
}

/// Form data to delete an existing group by identifier.
#[derive(Deserialize)]
pub struct DeleteGroupForm {
    pub id: i32,
}

/// Assigns a recipient to a group.
#[derive(Deserialize)]
pub struct AssignGroupRecipientForm {
    pub recipient_id: i32,
    pub group_id: i32,
}
