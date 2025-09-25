use pushkind_common::services::errors::{ServiceError, ServiceResult};
use validator::Validate;

use crate::domain::group::{Group, GroupWithRecipients};
use crate::domain::recipient::Recipient;
use crate::forms::groups::{AddGroupForm, AssignGroupRecipientForm, DeleteGroupForm};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader,
};

/// Data required to render the groups overview page.
pub struct GroupsOverviewData {
    pub groups: Vec<Group>,
    pub custom_fields: Vec<String>,
    pub recipients: Vec<Recipient>,
}

/// Service encapsulating group operations.
pub struct GroupsService<'a, R>
where
    R: GroupReader + GroupWriter + RecipientReader,
{
    repo: &'a R,
}

impl<'a, R> GroupsService<'a, R>
where
    R: GroupReader + GroupWriter + RecipientReader,
{
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    /// Loads the data required to render the groups overview page.
    pub fn load_overview(&self, hub_id: i32) -> ServiceResult<GroupsOverviewData> {
        let groups_query = GroupListQuery::new(hub_id);
        let (_, groups) = self.repo.list_groups(groups_query)?;

        let recipients_query = RecipientListQuery::new(hub_id);
        let (_, recipients) = self.repo.list_recipients(recipients_query)?;

        let custom_fields = self.repo.list_custom_fields(hub_id)?;

        Ok(GroupsOverviewData {
            groups,
            custom_fields,
            recipients,
        })
    }

    /// Creates a new group.
    pub fn create_group(&self, hub_id: i32, form: AddGroupForm) -> ServiceResult<()> {
        form.validate()
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        let new_group = form.to_new_group(hub_id);
        self.repo.create_group(&new_group)?;
        Ok(())
    }

    /// Deletes the specified group if it belongs to the hub.
    pub fn delete_group(&self, hub_id: i32, form: DeleteGroupForm) -> ServiceResult<()> {
        let group = self
            .repo
            .get_group_by_id(form.id, hub_id)?
            .ok_or(ServiceError::NotFound)?;

        self.repo.delete_group(group.group.id)?;
        Ok(())
    }

    /// Assigns recipients to a group.
    pub fn assign_recipients(&self, hub_id: i32, payload: &[u8]) -> ServiceResult<()> {
        let form: AssignGroupRecipientForm = serde_html_form::from_bytes(payload)
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        let group = self
            .repo
            .get_group_by_id(form.group_id, hub_id)?
            .ok_or(ServiceError::NotFound)?;

        self.repo
            .assign_recipients_to_group(group.group.id, form.recipient_id)?;
        Ok(())
    }

    /// Loads the data required to render the modal with group details.
    pub fn load_modal(&self, hub_id: i32, group_id: i32) -> ServiceResult<GroupWithRecipients> {
        let group = self
            .repo
            .get_group_by_id(group_id, hub_id)?
            .ok_or(ServiceError::NotFound)?;

        Ok(group)
    }
}
