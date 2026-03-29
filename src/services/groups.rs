//! Business logic for group management.
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::domain::group::GroupWithRecipients;
use crate::domain::types::{GroupId, HubId};
use crate::dto::groups::GroupsOverviewData;
use crate::forms::groups::{AddGroupPayload, AssignGroupRecipientPayload};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader,
};
use crate::services::ensure_emailer;

/// Loads the data required to render the groups overview page.
pub fn load_groups_overview<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<GroupsOverviewData>
where
    R: GroupReader + RecipientReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let groups_query = GroupListQuery::new(hub_id);
    let (_, groups) = repo.list_groups(groups_query)?;

    let recipients_query = RecipientListQuery::new(hub_id);
    let (_, recipients) = repo.list_recipients(recipients_query)?;

    let custom_fields = repo.list_custom_fields(hub_id)?;

    Ok(GroupsOverviewData {
        groups,
        custom_fields,
        recipients,
    })
}

/// Creates a new group.
pub fn create_group<R>(
    payload: AddGroupPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: GroupWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let new_group = payload.into_domain(hub_id);
    repo.create_group(&new_group)?;
    Ok(())
}

/// Deletes the specified group if it belongs to the hub.
pub fn delete_group<R>(group_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: GroupReader + GroupWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let group_id = GroupId::new(group_id)?;

    repo.delete_group(group_id, hub_id)?;
    Ok(())
}

/// Assigns recipients to a group.
pub fn assign_recipients<R>(
    group_id: i32,
    payload: AssignGroupRecipientPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: GroupReader + GroupWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let group_id = GroupId::new(group_id)?;

    repo.assign_recipients_to_group(group_id, payload.recipient_id, hub_id)?;
    Ok(())
}

/// Loads the data required to render the modal with group details.
pub fn load_group_modal<R>(
    group_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<GroupWithRecipients>
where
    R: GroupReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let group_id = GroupId::new(group_id)?;

    let group = repo
        .get_group_by_id(group_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    Ok(group)
}
