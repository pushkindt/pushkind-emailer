//! Email composition and sending form types.
use std::collections::HashSet;

use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile, text::Text};
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
use serde::Deserialize;
use validator::Validate;

use crate::domain::email::{NewEmail, NewEmailRecipient};
use crate::domain::types::RecipientEmail;
use crate::{
    repository::{EmailReader, EmailRecipientReader, RecipientListQuery, RecipientReader},
    utils::read_attachment_file,
};

/// Form data for sending a new email with optional attachment.
#[derive(MultipartForm)]
pub struct SendEmailForm {
    pub message: Text<String>,
    pub subject: Text<Option<String>>,
    pub cooldown_days: Text<Option<i64>>,
    #[multipart(limit = "10MB")]
    pub attachment: Option<TempFile>,
    pub recipients: MpJson<Vec<String>>,
}

/// Form data to remove an existing email.
#[derive(Deserialize, Validate)]
pub struct DeleteEmailForm {
    #[validate(range(min = 1))]
    pub id: i32,
}

/// Form data to resend an existing email (only unsent).
#[derive(Deserialize, Validate)]
pub struct ResendEmailForm {
    #[validate(range(min = 1))]
    pub id: i32,
}

impl SendEmailForm {
    /// Converts a [`SendEmailForm`] into the domain [`NewEmail`] type.
    pub fn to_new_email<R>(mut self, hub_id: i32, repo: &R) -> RepositoryResult<NewEmail>
    where
        R: RecipientReader + EmailRecipientReader + EmailReader,
    {
        let cooldown_days = self.cooldown_days.0;
        let (attachment_name, attachment_mime, attachment) = if let Some(attachment) =
            self.attachment.as_mut()
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

        let mut emails: Vec<String> = vec![];
        let mut groups: Vec<i32> = vec![];

        for address in self.recipients.0 {
            match address.parse::<i32>() {
                Ok(group_id) => groups.push(group_id),
                Err(_) => emails.push(address),
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
            Err(e) => return Err(e),
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
            Err(e) => return Err(e),
        };

        recipients.extend(individual_recipients);

        recipients.sort_by(|a, b| a.address.as_str().cmp(b.address.as_str()));
        recipients.dedup_by(|a, b| a.address == b.address);

        if let Some(days) = cooldown_days.filter(|d| *d > 0) {
            let recent_addresses: HashSet<RecipientEmail> = repo
                .list_recent_recipients(hub_id, Some(days))?
                .into_iter()
                .map(|recipient| recipient.address)
                .collect();

            if !recent_addresses.is_empty() {
                recipients.retain(|recipient| !recent_addresses.contains(&recipient.address));
            }
        }

        NewEmail::try_new(
            hub_id,
            ammonia::clean(&self.message.0),
            self.subject.0,
            attachment,
            attachment_name,
            attachment_mime,
            recipients,
        )
        .map_err(|err| RepositoryError::ValidationError(err.to_string()))
    }
}
