//! Group-related form types and input validation.
use serde::Deserialize;
use validator::Validate;

use crate::{
    domain::{
        group::NewGroup,
        types::{GroupName, HubId, RecipientId, TypeConstraintError},
    },
    forms::FormError,
};

/// Form data for creating a new recipient group.
#[derive(Deserialize, Validate)]
pub struct AddGroupForm {
    #[validate(length(min = 1))]
    pub name: String,
}

pub struct AddGroupPayload {
    pub name: GroupName,
}

/// Assigns a recipient to a group.
#[derive(Deserialize, Validate)]
pub struct AssignGroupRecipientForm {
    #[serde(default)]
    pub recipient_id: Vec<i32>,
}

pub struct AssignGroupRecipientPayload {
    pub recipient_id: Vec<RecipientId>,
}

impl TryFrom<AddGroupForm> for AddGroupPayload {
    type Error = FormError;

    fn try_from(form: AddGroupForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            name: GroupName::new(form.name).map_err(|_| FormError::InvalidName)?,
        })
    }
}

impl AddGroupPayload {
    pub fn into_domain(self, hub_id: HubId) -> NewGroup {
        NewGroup {
            name: self.name,
            hub_id,
        }
    }
}

impl TryFrom<AssignGroupRecipientForm> for AssignGroupRecipientPayload {
    type Error = FormError;

    fn try_from(form: AssignGroupRecipientForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            recipient_id: form
                .recipient_id
                .into_iter()
                .map(RecipientId::new)
                .collect::<Result<Vec<RecipientId>, TypeConstraintError>>()
                .map_err(|_| FormError::InvalidRecipientId)?,
        })
    }
}
