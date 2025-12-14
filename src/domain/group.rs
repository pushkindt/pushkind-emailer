//! Domain types for recipient groups.
use chrono::NaiveDateTime;
use serde::Serialize;

use crate::domain::recipient::Recipient;
use crate::domain::types::{GroupId, HubId, NonEmptyString, TypeConstraintError};

#[derive(Serialize, Debug)]
/// A named collection of recipients within a hub.
pub struct Group {
    /// Identifier of the group.
    pub id: GroupId,
    /// Display name of the group.
    pub name: NonEmptyString,
    /// Hub that owns the group.
    pub hub_id: HubId,
    /// Time the group was created.
    pub created_at: Option<NaiveDateTime>,
    /// Last modification time of the group.
    pub updated_at: Option<NaiveDateTime>,
}

impl Group {
    #[must_use]
    pub const fn new(
        id: GroupId,
        name: NonEmptyString,
        hub_id: HubId,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id,
            name,
            hub_id,
            created_at,
            updated_at,
        }
    }

    pub fn try_new(
        id: i32,
        name: impl Into<String>,
        hub_id: i32,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            id: GroupId::try_from(id)?,
            name: NonEmptyString::new(name.into())?,
            hub_id: HubId::try_from(hub_id)?,
            created_at,
            updated_at,
        })
    }
}

#[derive(Serialize)]
/// A group together with all of its recipients.
pub struct GroupWithRecipients {
    /// The group metadata.
    pub group: Group,
    /// Recipients belonging to the group.
    pub recipients: Vec<Recipient>,
}

/// Parameters to create a new [`Group`].
pub struct NewGroup {
    /// Name of the group.
    pub name: NonEmptyString,
    /// Hub that will own the group.
    pub hub_id: HubId,
}

impl NewGroup {
    #[must_use]
    pub const fn new(name: NonEmptyString, hub_id: HubId) -> Self {
        Self { name, hub_id }
    }

    pub fn try_new(name: impl Into<String>, hub_id: i32) -> Result<Self, TypeConstraintError> {
        Ok(Self {
            name: NonEmptyString::new(name.into())?,
            hub_id: HubId::try_from(hub_id)?,
        })
    }
}
