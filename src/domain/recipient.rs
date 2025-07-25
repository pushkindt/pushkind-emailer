use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::group::Group;

#[derive(Serialize)]
/// An individual that can receive emails from a hub.
pub struct Recipient {
    /// Database identifier of the recipient.
    pub id: i32,
    /// Human readable name.
    pub name: String,
    /// Email address used for deliveries.
    pub email: String,
    /// Hub this recipient belongs to.
    pub hub_id: i32,
    /// Additional custom fields associated with the recipient.
    pub fields: HashMap<String, String>,
    /// Time the recipient was created.
    pub created_at: Option<NaiveDateTime>,
    /// Last modification time.
    pub updated_at: Option<NaiveDateTime>,
    /// When the recipient unsubscribed, if applicable.
    pub unsubscribed_at: Option<NaiveDateTime>,
    /// Groups to which the recipient belongs.
    pub groups: Vec<i32>,
}

#[derive(Serialize)]
/// A recipient together with the groups they are a member of.
pub struct RecipientWithGroups {
    /// The recipient details.
    pub recipient: Recipient,
    /// Groups associated with the recipient.
    pub groups: Vec<Group>,
}

/// Data required to create a new [`Recipient`].
#[derive(Deserialize)]
pub struct NewRecipient {
    /// Name of the recipient.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Hub to associate with the recipient.
    pub hub_id: i32,
    /// Optional set of custom fields.
    pub fields: Option<HashMap<String, String>>,
    /// Optional list of group names to subscribe the recipient to.
    pub groups: Option<Vec<String>>,
}

/// Fields that can be modified for an existing [`Recipient`].
pub struct UpdateRecipient {
    /// Updated name.
    pub name: String,
    /// Updated email address.
    pub email: String,
    /// Updated map of custom fields.
    pub fields: HashMap<String, String>,
    /// Timestamp when the recipient unsubscribed.
    pub unsubscribed_at: Option<NaiveDateTime>,
    /// Groups the recipient should belong to.
    pub groups: Vec<i32>,
}
