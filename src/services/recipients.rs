//! Business logic for recipient management and importing.
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use validator::Validate;

use crate::domain::recipient::NewRecipient;
use crate::domain::types::TypeConstraintError;
use crate::dto::recipients::{RecipientModalData, RecipientsOverviewData};
use crate::forms::recipients::{
    AddRecipientForm, DeleteRecipientForm, SaveRecipientForm, SourceRecipientForm,
    UploadRecipientsForm,
};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader, RecipientWriter,
};
use crate::services::authorization::ensure_emailer;
use crate::utils::calculate_total_pages;

/// Loads the data required to render the recipients overview page.
pub fn load_recipients_overview<R>(
    repo: &R,
    user: &AuthenticatedUser,
    page: usize,
    query: Option<String>,
) -> ServiceResult<RecipientsOverviewData>
where
    R: RecipientReader,
{
    ensure_emailer(user)?;

    let normalized_query = query.and_then(|q| {
        let trimmed = q.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let list_query = RecipientListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);
    let list_query = if let Some(ref search) = normalized_query {
        list_query.search(search)
    } else {
        list_query
    };

    let (total, recipients) = repo.list_recipients(list_query)?;

    let total_pages = calculate_total_pages(total, DEFAULT_ITEMS_PER_PAGE);
    let recipients = Paginated::new(recipients, page, total_pages);

    Ok(RecipientsOverviewData {
        recipients,
        search_query: normalized_query,
    })
}

/// Creates a new recipient from the provided form.
pub fn create_recipient<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddRecipientForm,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let new_recipient = NewRecipient::try_new(form.name, form.email, user.hub_id, None, None)
        .map_err(|err| {
            log::error!("Invalid recipient payload: {err}");
            ServiceError::Form(err.to_string())
        })?;
    repo.create_recipients(&[new_recipient])?;
    Ok(())
}

/// Deletes a recipient belonging to the hub.
pub fn delete_recipient<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: DeleteRecipientForm,
) -> ServiceResult<()>
where
    R: RecipientReader + RecipientWriter,
{
    ensure_emailer(user)?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let recipient = repo
        .get_recipient_by_id(form.id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    repo.delete_recipient(recipient.recipient.id.get())?;
    Ok(())
}

/// Removes all groups and recipients for the hub.
pub fn clean_recipients<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<()>
where
    R: GroupWriter + RecipientWriter,
{
    ensure_emailer(user)?;

    repo.delete_all_groups(user.hub_id)?;
    repo.delete_all_recipients(user.hub_id)?;
    Ok(())
}

/// Parses and persists recipients uploaded via CSV.
pub fn upload_recipients<R>(
    repo: &R,
    user: &AuthenticatedUser,
    mut form: UploadRecipientsForm,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    let recipients = form
        .parse(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_recipients(&recipients)?;
    Ok(())
}

/// Loads the data required to render the recipient modal dialog.
pub fn load_recipient_modal<R>(
    repo: &R,
    user: &AuthenticatedUser,
    recipient_id: i32,
) -> ServiceResult<RecipientModalData>
where
    R: RecipientReader + GroupReader,
{
    ensure_emailer(user)?;

    let recipient = repo
        .get_recipient_by_id(recipient_id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let groups_query = GroupListQuery::new(user.hub_id);
    let (_, groups) = repo.list_groups(groups_query)?;

    Ok(RecipientModalData { recipient, groups })
}

/// Saves recipient changes from an HTML form payload.
pub fn save_recipient<R>(repo: &R, user: &AuthenticatedUser, payload: &[u8]) -> ServiceResult<()>
where
    R: RecipientReader + RecipientWriter,
{
    ensure_emailer(user)?;

    let form: SaveRecipientForm =
        serde_html_form::from_bytes(payload).map_err(|err| ServiceError::Form(err.to_string()))?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let recipient = repo
        .get_recipient_by_id(form.id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?
        .recipient;

    let updates = form
        .try_into_update_recipient()
        .map_err(|err: TypeConstraintError| ServiceError::Form(err.to_string()))?;
    repo.update_recipient(recipient.id.get(), &updates)?;
    Ok(())
}

/// Loads recipients from an external source and persists them.
pub async fn import_recipients_from_source<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: SourceRecipientForm,
    cookie_value: &str,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let recipients = form
        .load(cookie_value, user.hub_id)
        .await
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_recipients(&recipients)?;
    Ok(())
}
