//! Business logic for group management.
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use validator::Validate;

use crate::domain::group::GroupWithRecipients;
use crate::dto::groups::GroupsOverviewData;
use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader,
};
use crate::services::ensure_emailer;

/// Loads the data required to render the groups overview page.
pub fn load_groups_overview<R>(
    repo: &R,
    user: &AuthenticatedUser,
) -> ServiceResult<GroupsOverviewData>
where
    R: GroupReader + RecipientReader,
{
    ensure_emailer(user)?;

    let hub_id = user.hub_id;
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
pub fn create_group<R>(repo: &R, user: &AuthenticatedUser, form: AddGroupForm) -> ServiceResult<()>
where
    R: GroupWriter,
{
    ensure_emailer(user)?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let new_group = form
        .to_new_group(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;
    repo.create_group(&new_group)?;
    Ok(())
}

/// Deletes the specified group if it belongs to the hub.
pub fn delete_group<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: DeleteGroupForm,
) -> ServiceResult<()>
where
    R: GroupReader + GroupWriter,
{
    ensure_emailer(user)?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let group = repo
        .get_group_by_id(form.id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    repo.delete_group(group.group.id.get())?;
    Ok(())
}

/// Assigns recipients to a group.
pub fn assign_recipients<R>(repo: &R, user: &AuthenticatedUser, payload: &[u8]) -> ServiceResult<()>
where
    R: GroupReader + GroupWriter,
{
    ensure_emailer(user)?;

    let form: AssignGroupRecipientForm =
        serde_html_form::from_bytes(payload).map_err(|err| ServiceError::Form(err.to_string()))?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let group = repo
        .get_group_by_id(form.group_id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    repo.assign_recipients_to_group(group.group.id.get(), form.recipient_id)?;
    Ok(())
}

/// Loads the data required to render the modal with group details.
pub fn load_group_modal<R>(
    repo: &R,
    user: &AuthenticatedUser,
    group_id: i32,
) -> ServiceResult<GroupWithRecipients>
where
    R: GroupReader,
{
    ensure_emailer(user)?;

    let group = repo
        .get_group_by_id(group_id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    Ok(group)
}
