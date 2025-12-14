use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::email::UpdateEmailRecipient;
use pushkind_common::models::emailer::zmq::ZMQSendEmailMessage;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use pushkind_common::zmq::{ZmqSender, ZmqSenderExt};

use crate::domain::recipient::CSVExportRecipient;
use crate::dto::main::{ExportedEmailRecipients, IndexPageData};
use crate::forms::main::{DeleteEmailForm, ResendEmailForm, SendEmailForm};
use crate::repository::{
    EmailListQuery, EmailReader, EmailRecipientReader, EmailWriter, GroupListQuery, GroupReader,
    RecipientListQuery, RecipientReader,
};

/// Loads the data required to render the index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    retry_id: Option<i32>,
    page: usize,
) -> ServiceResult<IndexPageData>
where
    R: EmailReader + RecipientReader + GroupReader,
{
    ensure_emailer(user)?;

    let hub_id = user.hub_id;
    let retry_email = match retry_id {
        Some(id) => repo.get_email_by_id(id, hub_id)?,
        None => None,
    };

    let recipients_query = RecipientListQuery::new(hub_id);
    let (_, recipients) = repo.list_recipients(recipients_query)?;

    let groups_query = GroupListQuery::new(hub_id);
    let (_, groups) = repo.list_groups(groups_query)?;

    let emails_query = EmailListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);
    let (total, emails) = repo.list_emails(emails_query)?;
    let total_pages = calculate_total_pages(total, DEFAULT_ITEMS_PER_PAGE);
    let emails = Paginated::new(emails, page, total_pages);

    let custom_fields = repo.list_custom_fields(hub_id)?;

    Ok(IndexPageData {
        retry_email,
        recipients,
        groups,
        emails,
        custom_fields,
    })
}

/// Queues a new email for delivery via ZeroMQ.
pub async fn queue_new_email<R>(
    repo: &R,
    user: AuthenticatedUser,
    form: SendEmailForm,
    zmq_sender: &ZmqSender,
) -> ServiceResult<()>
where
    R: RecipientReader + EmailRecipientReader + EmailReader,
{
    ensure_emailer(&user)?;

    let new_email = form.to_new_email(user.hub_id, repo)?;

    if new_email.recipients.is_empty() {
        return Err(ServiceError::Form("Не указаны получатели.".into()));
    }

    let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user, new_email)));
    zmq_sender.send_json(&zmq_message).await?;
    Ok(())
}

/// Deletes an email belonging to the provided hub.
pub fn delete_email<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: DeleteEmailForm,
) -> ServiceResult<()>
where
    R: EmailReader + EmailWriter,
{
    ensure_emailer(user)?;

    let email = repo
        .get_email_by_id(form.id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    repo.delete_email(email.email.id)?;
    Ok(())
}

/// Re-queues an email for delivery.
pub async fn queue_email_retry<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: ResendEmailForm,
    zmq_sender: &ZmqSender,
) -> ServiceResult<()>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let email = repo
        .get_email_by_id(form.id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let zmq_message = ZMQSendEmailMessage::RetryEmail((email.email.id, user.hub_id));
    zmq_sender.send_json(&zmq_message).await?;
    Ok(())
}

/// Marks a recipient as having opened an email.
pub fn mark_email_opened<R>(repo: &R, recipient_id: i32) -> ServiceResult<()>
where
    R: EmailWriter,
{
    repo.update_recipient(
        recipient_id,
        &UpdateEmailRecipient {
            opened: Some(true),
            is_sent: Some(true),
            replied: None,
            reply: None,
        },
    )?;

    Ok(())
}

/// Exports all recipients of an email as a CSV payload.
pub fn export_email_recipients<R>(
    repo: &R,
    user: &AuthenticatedUser,
    email_id: i32,
) -> ServiceResult<ExportedEmailRecipients>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let email = repo
        .get_email_by_id(email_id, user.hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let mut writer = csv::Writer::from_writer(vec![]);
    for recipient in email.recipients {
        let recipient = CSVExportRecipient::from(recipient);
        writer.serialize(recipient).map_err(|err| {
            log::error!("Failed to serialize recipient: {err}");
            ServiceError::Internal
        })?;
    }

    let data = writer.into_inner().map_err(|err| {
        log::error!("Failed to finalize csv: {err}");
        ServiceError::Internal
    })?;

    Ok(ExportedEmailRecipients {
        filename: format!("recipients_{email_id}.csv"),
        bytes: data,
    })
}

fn calculate_total_pages(total_items: usize, per_page: usize) -> usize {
    if per_page == 0 {
        return 0;
    }

    total_items.div_ceil(per_page)
}

fn ensure_emailer(user: &AuthenticatedUser) -> ServiceResult<()> {
    if check_role("emailer", &user.roles) {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}
