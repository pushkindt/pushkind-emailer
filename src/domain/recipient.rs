//! Domain types for recipients and subscriptions.
use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::domain::email::EmailRecipient;
use crate::domain::group::Group;
use crate::domain::types::{
    GroupId, HubId, RecipientEmail, RecipientId, RecipientName, TypeConstraintError,
    UnsubscribeReason,
};

#[derive(Serialize, Debug)]
/// An individual that can receive emails from a hub.
pub struct Recipient {
    /// Database identifier of the recipient.
    pub id: RecipientId,
    /// Human readable name.
    pub name: RecipientName,
    /// Email address used for deliveries.
    pub email: RecipientEmail,
    /// Hub this recipient belongs to.
    pub hub_id: HubId,
    /// Additional custom fields associated with the recipient.
    pub fields: BTreeMap<String, String>,
    /// Time the recipient was created.
    pub created_at: Option<NaiveDateTime>,
    /// Last modification time.
    pub updated_at: Option<NaiveDateTime>,
    /// When the recipient unsubscribed, if applicable.
    pub unsubscribed_at: Option<NaiveDateTime>,
    /// Groups to which the recipient belongs.
    pub groups: Vec<GroupId>,
}

impl Recipient {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: RecipientId,
        name: RecipientName,
        email: RecipientEmail,
        hub_id: HubId,
        fields: BTreeMap<String, String>,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
        unsubscribed_at: Option<NaiveDateTime>,
        groups: Vec<GroupId>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            hub_id,
            fields,
            created_at,
            updated_at,
            unsubscribed_at,
            groups,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        name: impl Into<String>,
        email: impl Into<String>,
        hub_id: i32,
        fields: BTreeMap<String, String>,
        created_at: Option<NaiveDateTime>,
        updated_at: Option<NaiveDateTime>,
        unsubscribed_at: Option<NaiveDateTime>,
        groups: Vec<i32>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            RecipientId::try_from(id)?,
            RecipientName::new(name.into())?,
            RecipientEmail::new(email.into())?,
            HubId::try_from(hub_id)?,
            fields,
            created_at,
            updated_at,
            unsubscribed_at,
            groups
                .into_iter()
                .map(GroupId::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
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
pub struct NewRecipient {
    /// Name of the recipient.
    pub name: RecipientName,
    /// Email address.
    pub email: RecipientEmail,
    /// Hub to associate with the recipient.
    pub hub_id: HubId,
    /// Optional set of custom fields.
    pub fields: Option<BTreeMap<String, String>>,
    /// Optional list of group names to subscribe the recipient to.
    pub groups: Option<Vec<String>>,
}

impl NewRecipient {
    #[must_use]
    pub fn new(
        name: RecipientName,
        email: RecipientEmail,
        hub_id: HubId,
        fields: Option<BTreeMap<String, String>>,
        groups: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            email,
            hub_id,
            fields,
            groups,
        }
    }

    pub fn try_new(
        name: impl Into<String>,
        email: impl Into<String>,
        hub_id: i32,
        fields: Option<BTreeMap<String, String>>,
        groups: Option<Vec<String>>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            RecipientName::new(name.into())?,
            RecipientEmail::new(email.into())?,
            HubId::try_from(hub_id)?,
            fields,
            groups,
        ))
    }
}

/// Fields that can be modified for an existing [`Recipient`].
pub struct UpdateRecipient {
    /// Updated name.
    pub name: RecipientName,
    /// Updated email address.
    pub email: RecipientEmail,
    /// Updated map of custom fields.
    pub fields: BTreeMap<String, String>,
    /// Groups the recipient should belong to.
    pub groups: Vec<GroupId>,
}

impl UpdateRecipient {
    #[must_use]
    pub fn new(
        name: RecipientName,
        email: RecipientEmail,
        fields: BTreeMap<String, String>,
        groups: Vec<GroupId>,
    ) -> Self {
        Self {
            name,
            email,
            fields,
            groups,
        }
    }

    pub fn try_new(
        name: impl Into<String>,
        email: impl Into<String>,
        fields: BTreeMap<String, String>,
        groups: Vec<i32>,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            RecipientName::new(name.into())?,
            RecipientEmail::new(email.into())?,
            fields,
            groups
                .into_iter()
                .map(GroupId::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

#[derive(Serialize)]
pub struct Unsubscribe {
    pub email: RecipientEmail,
    pub hub_id: HubId,
    pub reason: Option<UnsubscribeReason>,
    pub unsubscribed_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct CSVExportRecipient {
    pub email: String,
    pub name: String,
    pub opened: bool,
    pub sent: bool,
    pub replied: bool,
    pub updated_at: NaiveDateTime,
}

impl From<EmailRecipient> for CSVExportRecipient {
    fn from(value: EmailRecipient) -> Self {
        Self {
            email: value.address.into_inner(),
            name: value.name.into_inner(),
            opened: value.opened,
            sent: value.is_sent,
            replied: value.replied,
            updated_at: value.updated_at,
        }
    }
}
