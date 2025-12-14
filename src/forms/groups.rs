use serde::Deserialize;
use validator::Validate;

use crate::domain::group::NewGroup;
use crate::domain::types::TypeConstraintError;

/// Form data for creating a new recipient group.
#[derive(Deserialize, Validate)]
pub struct AddGroupForm {
    #[validate(length(min = 1))]
    pub name: String,
}

/// Form data to delete an existing group by identifier.
#[derive(Deserialize, Validate)]
pub struct DeleteGroupForm {
    #[validate(range(min = 1))]
    pub id: i32,
}

/// Assigns a recipient to a group.
#[derive(Deserialize, Validate)]
pub struct AssignGroupRecipientForm {
    #[serde(default)]
    pub recipient_id: Vec<i32>,
    #[validate(range(min = 1))]
    pub group_id: i32,
}

impl AddGroupForm {
    pub fn to_new_group(&self, hub_id: i32) -> Result<NewGroup, TypeConstraintError> {
        NewGroup::try_new(self.name.clone(), hub_id)
    }
}
