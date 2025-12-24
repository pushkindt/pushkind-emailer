//! Business logic for email composition, sending, and tracking.
use std::collections::HashSet;

use crate::domain::email::{NewEmail, NewEmailRecipient, UpdateEmailRecipient};
use crate::domain::types::{EmailId, GroupId, HubId, RecipientEmail, RecipientId};
use crate::models::zmq::ZMQSendEmailMessage;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use pushkind_common::zmq::{ZmqSender, ZmqSenderExt};

use crate::domain::recipient::CSVExportRecipient;
use crate::dto::main::{ExportedEmailRecipients, IndexPageData, IndexQueryParams};
use crate::forms::main::SendEmailForm;
use crate::repository::{
    EmailListQuery, EmailReader, EmailWriter, GroupListQuery, GroupReader, RecipientListQuery,
    RecipientReader,
};
use crate::services::ensure_emailer;
use crate::utils::{calculate_total_pages, read_attachment_file};

/// Converts a [`SendEmailForm`] into the domain [`NewEmail`] type.
pub fn new_email<R>(form: SendEmailForm, hub_id: HubId, repo: &R) -> ServiceResult<NewEmail>
where
    R: RecipientReader + EmailReader,
{
    let mut form = form;
    let cooldown_days = form.cooldown_days.0;
    let (attachment_name, attachment_mime, attachment) = if let Some(attachment) =
        form.attachment.as_mut()
        && attachment
            .file_name
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .is_some()
        && attachment.content_type.is_some()
    {
        match read_attachment_file(attachment) {
            Ok((name, mime, data)) => (name, mime, data),
            Err(err) => {
                log::error!("error reading file: {err}");
                (None, None, None)
            }
        }
    } else {
        (None, None, None)
    };

    let mut emails: Vec<RecipientEmail> = vec![];
    let mut groups: Vec<GroupId> = vec![];

    for address in form.recipients.0 {
        match address.parse::<i32>() {
            Ok(group_id) => {
                let group_id = GroupId::new(group_id)
                    .map_err(|_| ServiceError::Form("Некорректный идентификатор группы.".into()))?;
                groups.push(group_id);
            }
            Err(_) => {
                let email = RecipientEmail::new(address)
                    .map_err(|_| ServiceError::Form("Некорректный email получателя.".into()))?;
                emails.push(email);
            }
        }
    }

    let mut recipients: Vec<NewEmailRecipient> = vec![];

    let query = RecipientListQuery::new(hub_id).group_ids(groups);

    let group_recipients: Vec<NewEmailRecipient> = match repo.list_recipients(query) {
        Ok((_total, groups)) => groups
            .into_iter()
            .filter(|recipient| recipient.unsubscribed_at.is_none())
            .map(|recipient| NewEmailRecipient {
                address: recipient.email,
                name: recipient.name,
                fields: recipient.fields,
            })
            .collect(),
        Err(e) => return Err(e.into()),
    };

    recipients.extend(group_recipients);

    let query = RecipientListQuery::new(hub_id).emails(emails);

    let individual_recipients: Vec<NewEmailRecipient> = match repo.list_recipients(query) {
        Ok((_total, recipients)) => recipients
            .into_iter()
            .filter(|recipient| recipient.unsubscribed_at.is_none())
            .map(|recipient| NewEmailRecipient {
                address: recipient.email,
                name: recipient.name,
                fields: recipient.fields,
            })
            .collect(),
        Err(e) => return Err(e.into()),
    };

    recipients.extend(individual_recipients);

    recipients.sort_by(|a, b| a.address.as_str().cmp(b.address.as_str()));
    recipients.dedup_by(|a, b| a.address == b.address);

    if let Some(days) = cooldown_days.filter(|d| *d > 0) {
        let recent_addresses: HashSet<RecipientEmail> = repo
            .list_recent_email_recipients(hub_id, Some(days))?
            .into_iter()
            .map(|recipient| recipient.address)
            .collect();

        if !recent_addresses.is_empty() {
            recipients.retain(|recipient| !recent_addresses.contains(&recipient.address));
        }
    }

    let email = NewEmail::try_new(
        hub_id.get(),
        ammonia::clean(&form.message.0),
        form.subject.0,
        attachment,
        attachment_name,
        attachment_mime,
        recipients,
    )?;

    Ok(email)
}

/// Loads the data required to render the index page.
pub fn load_index_page<R>(
    params: IndexQueryParams,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<IndexPageData>
where
    R: EmailReader + RecipientReader + GroupReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;
    let retry_email = match params.retry {
        Some(id) => repo.get_email_by_id(EmailId::new(id)?, hub_id)?,
        None => None,
    };

    let page = params.page.unwrap_or(1);

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
    form: SendEmailForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &ZmqSender,
) -> ServiceResult<()>
where
    R: RecipientReader + EmailReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let new_email = new_email(form, hub_id, repo)?;

    if new_email.recipients.is_empty() {
        return Err(ServiceError::Form("Не указаны получатели.".into()));
    }

    let zmq_message = ZMQSendEmailMessage::NewEmail(Box::new((user.clone(), new_email)));
    zmq_sender.send_json(&zmq_message).await?;
    Ok(())
}

/// Deletes an email belonging to the provided hub.
pub fn delete_email<R>(email_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: EmailReader + EmailWriter,
{
    ensure_emailer(user)?;

    let email_id = EmailId::new(email_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    repo.delete_email(email_id, hub_id)?;
    Ok(())
}

/// Re-queues an email for delivery.
pub async fn queue_email_retry<R>(
    email_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &ZmqSender,
) -> ServiceResult<()>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let email_id = EmailId::new(email_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    let email = repo
        .get_email_by_id(email_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let zmq_message = ZMQSendEmailMessage::RetryEmail((email.email.id.get(), user.hub_id));
    zmq_sender.send_json(&zmq_message).await?;
    Ok(())
}

/// Marks a recipient as having opened an email.
pub fn mark_email_opened<R>(recipient_id: i32, repo: &R) -> ServiceResult<()>
where
    R: EmailWriter,
{
    let recipient_id = RecipientId::new(recipient_id)?;

    repo.update_email_recipient(
        recipient_id,
        &UpdateEmailRecipient {
            opened: true,
            is_sent: true,
        },
    )?;

    Ok(())
}

/// Exports all recipients of an email as a CSV payload.
pub fn export_email_recipients<R>(
    email_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ExportedEmailRecipients>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let email_id = EmailId::new(email_id)?;
    let hub_id = HubId::new(user.hub_id)?;

    let email = repo
        .get_email_by_id(email_id, hub_id)?
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

#[cfg(test)]
mod tests {
    use super::new_email;
    use std::collections::BTreeMap;
    use std::io::{Seek, SeekFrom, Write};

    use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, text::Text};
    use chrono::Utc;
    use mockall::Sequence;
    use tempfile::NamedTempFile;

    use crate::domain::email::EmailRecipient;
    use crate::domain::recipient::Recipient;
    use crate::domain::types::{GroupId, HubId, RecipientEmail, RecipientId, RecipientName};
    use crate::forms::main::SendEmailForm;
    use crate::repository::RecipientListQuery;
    use crate::repository::mock::MockRepository;

    fn build_recipient(id: i32, email: &str, name: &str, hub_id: HubId) -> Recipient {
        Recipient::new(
            RecipientId::new(id).unwrap(),
            RecipientName::new(name).unwrap(),
            RecipientEmail::new(email).unwrap(),
            hub_id,
            BTreeMap::new(),
            None,
            None,
            None,
            vec![],
        )
    }

    #[test]
    fn new_email_builds_recipients_and_attachment() {
        let hub_id = HubId::new(1).unwrap();
        let group_id = GroupId::new(1).unwrap();

        let mut named = NamedTempFile::new().unwrap();
        write!(named, "hello").unwrap();
        named.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
        let attachment = TempFile {
            file: named,
            content_type: Some(mime::TEXT_PLAIN),
            file_name: Some("hello.txt".into()),
            size: 5,
        };

        let form = SendEmailForm {
            message: Text("Hi".to_string()),
            subject: Text(Some("Sub".to_string())),
            cooldown_days: Text(None),
            attachment: Some(attachment),
            recipients: MpJson(vec!["1".to_string(), "a@example.com".to_string()]),
        };

        let mut repo = MockRepository::new();
        let mut seq = Sequence::new();

        let hub_id_match = hub_id;
        let group_id_match = group_id;
        repo.expect_list_recipients()
            .withf(move |query: &RecipientListQuery| {
                query.hub_id == hub_id_match
                    && query
                        .group_ids
                        .as_ref()
                        .map(|ids| ids == &vec![group_id_match])
                        .unwrap_or(false)
            })
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok((
                    1,
                    vec![build_recipient(1, "group@example.com", "Group", hub_id)],
                ))
            });

        let hub_id_match = hub_id;
        repo.expect_list_recipients()
            .withf(move |query: &RecipientListQuery| {
                query.hub_id == hub_id_match
                    && query
                        .emails
                        .as_ref()
                        .map(|emails| {
                            emails == &vec![RecipientEmail::new("a@example.com").unwrap()]
                        })
                        .unwrap_or(false)
            })
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok((
                    1,
                    vec![build_recipient(2, "a@example.com", "Alice", hub_id)],
                ))
            });

        repo.expect_list_recent_email_recipients().times(0);

        let email = new_email(form, hub_id, &repo).unwrap();

        assert_eq!(email.message.as_str(), "Hi");
        assert_eq!(email.subject.as_ref().map(|s| s.as_str()), Some("Sub"));
        assert_eq!(
            email.attachment_name.as_ref().map(|s| s.as_str()),
            Some("hello.txt")
        );
        assert_eq!(
            email.attachment_mime.as_ref().map(|s| s.as_str()),
            Some("text/plain")
        );
        assert_eq!(email.attachment.as_deref().unwrap(), b"hello");
        assert_eq!(email.recipients.len(), 2);
    }

    #[test]
    fn new_email_excludes_recent_recipients() {
        let hub_id = HubId::new(1).unwrap();

        let form = SendEmailForm {
            message: Text("Body".to_string()),
            subject: Text(None),
            cooldown_days: Text(Some(3)),
            attachment: None,
            recipients: MpJson(vec![
                "recent@example.com".to_string(),
                "stale@example.com".to_string(),
            ]),
        };

        let mut repo = MockRepository::new();
        let mut seq = Sequence::new();

        let hub_id_match = hub_id;
        repo.expect_list_recipients()
            .withf(move |query: &RecipientListQuery| {
                query.hub_id == hub_id_match
                    && query.group_ids.as_ref().map(Vec::is_empty).unwrap_or(false)
            })
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok((0, vec![])));

        let hub_id_match = hub_id;
        repo.expect_list_recipients()
            .withf(move |query: &RecipientListQuery| {
                query.hub_id == hub_id_match
                    && query
                        .emails
                        .as_ref()
                        .map(|emails| emails.len() == 2)
                        .unwrap_or(false)
            })
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| {
                Ok((
                    2,
                    vec![
                        build_recipient(1, "recent@example.com", "Recent", hub_id),
                        build_recipient(2, "stale@example.com", "Stale", hub_id),
                    ],
                ))
            });

        repo.expect_list_recent_email_recipients()
            .withf(move |hub, days| *hub == hub_id && *days == Some(3))
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_, _| {
                Ok(vec![
                    EmailRecipient::try_new(
                        1,
                        1,
                        "recent@example.com",
                        false,
                        Utc::now().naive_utc(),
                        true,
                        false,
                        None,
                        "Recent",
                        BTreeMap::new(),
                    )
                    .unwrap(),
                ])
            });

        let email = new_email(form, hub_id, &repo).unwrap();

        assert_eq!(email.recipients.len(), 1);
        assert_eq!(
            email.recipients[0].address,
            RecipientEmail::new("stale@example.com").unwrap()
        );
    }
}
