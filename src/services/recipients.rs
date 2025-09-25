use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use validator::Validate;

use crate::domain::group::Group;
use crate::domain::recipient::{NewRecipient, Recipient, RecipientWithGroups};
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, SourceRecipientForm,
    UploadRecipientsForm,
};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader, RecipientWriter,
};

/// Data required to render the recipients overview page.
pub struct RecipientsOverviewData {
    pub recipients: Paginated<Recipient>,
    pub search_query: Option<String>,
}

/// Data required to render the recipient modal dialog.
pub struct RecipientModalData {
    pub recipient: RecipientWithGroups,
    pub groups: Vec<Group>,
}

/// Service encapsulating recipient operations.
pub struct RecipientsService<'a, R>
where
    R: RecipientReader + RecipientWriter + GroupReader + GroupWriter,
{
    repo: &'a R,
}

impl<'a, R> RecipientsService<'a, R>
where
    R: RecipientReader + RecipientWriter + GroupReader + GroupWriter,
{
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    /// Loads the data required to render the recipients overview page.
    pub fn load_overview(
        &self,
        hub_id: i32,
        page: usize,
        query: Option<String>,
    ) -> ServiceResult<RecipientsOverviewData> {
        let normalized_query = query.and_then(|q| {
            let trimmed = q.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        let list_query = RecipientListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);
        let list_query = if let Some(ref search) = normalized_query {
            list_query.search(search)
        } else {
            list_query
        };

        let (total, recipients) = if normalized_query.is_some() {
            self.repo.search_recipients(list_query)?
        } else {
            self.repo.list_recipients(list_query)?
        };

        let total_pages = calculate_total_pages(total, DEFAULT_ITEMS_PER_PAGE);
        let recipients = Paginated::new(recipients, page, total_pages);

        Ok(RecipientsOverviewData {
            recipients,
            search_query: normalized_query,
        })
    }

    /// Creates a new recipient from the provided form.
    pub fn create_recipient(&self, hub_id: i32, form: AddRecipientForm) -> ServiceResult<()> {
        form.validate()
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        let mut new_recipient: NewRecipient = form.into();
        new_recipient.hub_id = hub_id;

        self.repo.create_recipients(&[new_recipient])?;
        Ok(())
    }

    /// Deletes a recipient belonging to the hub.
    pub fn delete_recipient(&self, hub_id: i32, form: DeleteRecipientForm) -> ServiceResult<()> {
        let recipient = self
            .repo
            .get_recipient_by_id(form.id, hub_id)?
            .ok_or(ServiceError::NotFound)?;

        self.repo.delete_recipient(recipient.recipient.id)?;
        Ok(())
    }

    /// Removes all groups and recipients for the hub.
    pub fn clean(&self, hub_id: i32) -> ServiceResult<()> {
        self.repo.delete_all_groups(hub_id)?;
        self.repo.delete_all_recipients(hub_id)?;
        Ok(())
    }

    /// Parses and persists recipients uploaded via CSV.
    pub fn upload_recipients(
        &self,
        hub_id: i32,
        mut form: UploadRecipientsForm,
    ) -> ServiceResult<()> {
        let recipients = form
            .parse(hub_id)
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        self.repo.create_recipients(&recipients)?;
        Ok(())
    }

    /// Loads the data required to render the recipient modal dialog.
    pub fn load_modal(&self, hub_id: i32, recipient_id: i32) -> ServiceResult<RecipientModalData> {
        let recipient = self
            .repo
            .get_recipient_by_id(recipient_id, hub_id)?
            .ok_or(ServiceError::NotFound)?;

        let groups_query = GroupListQuery::new(hub_id);
        let (_, groups) = self.repo.list_groups(groups_query)?;

        Ok(RecipientModalData { recipient, groups })
    }

    /// Saves recipient changes from an HTML form payload.
    pub fn save_recipient(&self, hub_id: i32, payload: &[u8]) -> ServiceResult<()> {
        let form: SaveRecipientForm = serde_html_form::from_bytes(payload)
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        let recipient = self
            .repo
            .get_recipient_by_id(form.id, hub_id)?
            .ok_or(ServiceError::NotFound)?
            .recipient;

        self.repo.update_recipient(recipient.id, &form.into())?;
        Ok(())
    }

    /// Loads recipients from an external source and persists them.
    pub async fn import_from_source(
        &self,
        hub_id: i32,
        form: SourceRecipientForm,
        cookie_value: &str,
    ) -> ServiceResult<()> {
        form.validate()
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        let mut recipients = form
            .load(cookie_value)
            .await
            .map_err(|err| ServiceError::Form(err.to_string()))?;

        for recipient in &mut recipients {
            recipient.hub_id = hub_id;
        }

        self.repo.create_recipients(&recipients)?;
        Ok(())
    }
}

fn calculate_total_pages(total_items: usize, per_page: usize) -> usize {
    if per_page == 0 {
        return 0;
    }

    total_items.div_ceil(per_page)
}
