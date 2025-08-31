use chrono::NaiveDateTime;
use serde::Serialize;

use crate::domain::recipient::Recipient;

#[derive(Serialize)]
/// A named collection of recipients within a hub.
pub struct Group {
    /// Identifier of the group.
    pub id: i32,
    /// Display name of the group.
    pub name: String,
    /// Hub that owns the group.
    pub hub_id: i32,
    /// Time the group was created.
    pub created_at: Option<NaiveDateTime>,
    /// Last modification time of the group.
    pub updated_at: Option<NaiveDateTime>,
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
    pub name: String,
    /// Hub that will own the group.
    pub hub_id: i32,
}
