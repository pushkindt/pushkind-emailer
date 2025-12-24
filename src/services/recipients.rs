//! Business logic for recipient management and importing.
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::domain::types::{HubId, RecipientId};
use crate::dto::recipients::{RecipientModalData, RecipientsOverviewData, RecipientsQueryParams};
use crate::forms::recipients::{
    AddRecipientForm, AddRecipientPayload, SaveRecipientForm, SaveRecipientPayload,
    SourceRecipientForm, SourceRecipientPayload, UploadRecipientsForm,
};
use crate::repository::{
    GroupListQuery, GroupReader, GroupWriter, RecipientListQuery, RecipientReader, RecipientWriter,
};
use crate::services::ensure_emailer;
use crate::utils::calculate_total_pages;

/// Loads the data required to render the recipients overview page.
pub fn load_recipients_overview<R>(
    params: RecipientsQueryParams,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<RecipientsOverviewData>
where
    R: RecipientReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let page = params.page.unwrap_or(1);

    let list_query = RecipientListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);
    let list_query = if let Some(ref search) = params.q {
        list_query.search(search)
    } else {
        list_query
    };

    let (total, recipients) = repo.list_recipients(list_query)?;

    let total_pages = calculate_total_pages(total, DEFAULT_ITEMS_PER_PAGE);
    let recipients = Paginated::new(recipients, page, total_pages);

    Ok(RecipientsOverviewData {
        recipients,
        search_query: params.q,
    })
}

/// Creates a new recipient from the provided form.
pub fn create_recipient<R>(
    form: AddRecipientForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let payload: AddRecipientPayload = form.try_into()?;

    let new_recipient = payload.into_domain(hub_id);
    repo.create_recipients(&[new_recipient])?;
    Ok(())
}

/// Deletes a recipient belonging to the hub.
pub fn delete_recipient<R>(
    recipient_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: RecipientReader + RecipientWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let recipient_id = RecipientId::new(recipient_id)?;

    repo.delete_recipient(recipient_id, hub_id)?;
    Ok(())
}

/// Removes all groups and recipients for the hub.
pub fn clean_recipients<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: GroupWriter + RecipientWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    repo.delete_all_groups(hub_id)?;
    repo.delete_all_recipients(hub_id)?;
    Ok(())
}

/// Parses and persists recipients uploaded via CSV.
pub fn upload_recipients<R>(
    form: UploadRecipientsForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    let mut form = form;

    let recipients = form
        .parse(user.hub_id)
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_recipients(&recipients)?;
    Ok(())
}

/// Loads the data required to render the recipient modal dialog.
pub fn load_recipient_modal<R>(
    recipient_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<RecipientModalData>
where
    R: RecipientReader + GroupReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let recipient_id = RecipientId::new(recipient_id)?;

    let recipient = repo
        .get_recipient_by_id(recipient_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let groups_query = GroupListQuery::new(hub_id);
    let (_, groups) = repo.list_groups(groups_query)?;

    Ok(RecipientModalData { recipient, groups })
}

/// Saves recipient changes from an HTML form payload.
pub fn save_recipient<R>(
    recipient_id: i32,
    form: SaveRecipientForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<()>
where
    R: RecipientReader + RecipientWriter,
{
    ensure_emailer(user)?;

    let payload: SaveRecipientPayload = form.try_into()?;

    let hub_id = HubId::new(user.hub_id)?;
    let recipient_id = RecipientId::new(recipient_id)?;

    let updates = payload.into_domain();
    repo.update_recipient(recipient_id, hub_id, &updates)?;
    Ok(())
}

/// Loads recipients from an external source and persists them.
pub async fn import_recipients_from_source<R>(
    form: SourceRecipientForm,
    user: &AuthenticatedUser,
    repo: &R,
    cookie_value: &str,
) -> ServiceResult<()>
where
    R: RecipientWriter,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let payload: SourceRecipientPayload = form.try_into()?;

    let recipients = payload
        .load(cookie_value, hub_id)
        .await
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    repo.create_recipients(&recipients)?;
    Ok(())
}
