//! Integration tests covering email form validation and handling.
use std::{
    collections::HashMap,
    io::{Seek, SeekFrom, Write},
};

use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, text::Text};
use chrono::{Duration, NaiveDateTime, Utc};
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
use pushkind_emailer::{
    domain::email::{Email, EmailRecipient, EmailWithRecipients, NewEmail},
    domain::recipient::{Recipient, Unsubscribe},
    domain::types::{HubId, RecipientEmail, RecipientName},
    forms::main::SendEmailForm,
    repository::{
        EmailListQuery, EmailReader, EmailRecipientReader, RecipientListQuery, RecipientReader,
        test::TestRepository,
    },
};
use tempfile::NamedTempFile;

#[test]
fn send_email_form_into_new_email_with_attachment() {
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
        recipients: MpJson(vec!["a@example.com".to_string()]),
    };

    let email: NewEmail = form.to_new_email(1, &TestRepository {}).unwrap();

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
}

struct CooldownRepository {
    hub_id: i32,
    names: HashMap<String, String>,
    history: Vec<HistoryEntry>,
    email_created: HashMap<i32, NaiveDateTime>,
}

struct HistoryEntry {
    email_id: i32,
    address: String,
    is_sent: bool,
    sent_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl CooldownRepository {
    fn new(hub_id: i32, names: HashMap<String, String>, history: Vec<HistoryEntry>) -> Self {
        let email_created: HashMap<i32, NaiveDateTime> = history
            .iter()
            .map(|entry| (entry.email_id, entry.sent_at))
            .collect();

        Self {
            hub_id,
            names,
            history,
            email_created,
        }
    }

    fn recipients_for(&self, emails: Vec<String>) -> Vec<Recipient> {
        emails
            .into_iter()
            .filter_map(|address| {
                self.names.get(&address).map(|name| {
                    Recipient::new(
                        pushkind_emailer::domain::types::RecipientId::try_from(1).unwrap(),
                        RecipientName::try_from(name.as_str()).unwrap(),
                        RecipientEmail::try_from(address.as_str()).unwrap(),
                        HubId::try_from(self.hub_id).unwrap(),
                        HashMap::new(),
                        None,
                        None,
                        None,
                        vec![],
                    )
                })
            })
            .collect()
    }
}

impl RecipientReader for CooldownRepository {
    fn get_recipient_by_id(
        &self,
        _id: i32,
        _hub_id: i32,
    ) -> RepositoryResult<Option<pushkind_emailer::domain::recipient::RecipientWithGroups>> {
        Ok(None)
    }

    fn list_custom_fields(&self, _hub_id: i32) -> RepositoryResult<Vec<String>> {
        Ok(vec![])
    }

    fn list_recipients(
        &self,
        query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<Recipient>)> {
        if let Some(emails) = query.emails {
            let recipients = self.recipients_for(emails);
            let total = recipients.len();
            Ok((total, recipients))
        } else {
            Ok((0, vec![]))
        }
    }

    fn list_unsubscribed_recipients(&self, _hub_id: i32) -> RepositoryResult<Vec<Unsubscribe>> {
        Ok(vec![])
    }
}

impl EmailRecipientReader for CooldownRepository {
    fn list_recent_recipients(
        &self,
        hub_id: i32,
        number_of_days: Option<i64>,
    ) -> RepositoryResult<Vec<EmailRecipient>> {
        if hub_id != self.hub_id {
            return Ok(vec![]);
        }

        let cutoff = number_of_days
            .filter(|days| *days > 0)
            .map(|days| Utc::now().naive_utc() - Duration::days(days));

        let recipients = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, entry)| cutoff.map(|cutoff| entry.sent_at > cutoff).unwrap_or(true))
            .map(|(index, entry)| {
                EmailRecipient::try_new(
                    index as i32 + 1,
                    entry.email_id,
                    entry.address.clone(),
                    false,
                    entry.updated_at,
                    entry.is_sent,
                    false,
                    None,
                    self.names
                        .get(&entry.address)
                        .cloned()
                        .unwrap_or_else(|| entry.address.clone()),
                    HashMap::new(),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| RepositoryError::ValidationError(err.to_string()))?;

        Ok(recipients)
    }
}

impl EmailReader for CooldownRepository {
    fn get_email_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<EmailWithRecipients>> {
        if hub_id != self.hub_id {
            return Ok(None);
        }

        Ok(self
            .email_created
            .get(&id)
            .map(|created_at| EmailWithRecipients {
                email: Email::try_new(
                    id,
                    "Message",
                    *created_at,
                    true,
                    None,
                    None,
                    None,
                    None,
                    0,
                    0,
                    0,
                    self.hub_id,
                )
                .unwrap(),
                recipients: vec![],
            }))
    }

    fn list_emails(
        &self,
        _query: EmailListQuery,
    ) -> RepositoryResult<(usize, Vec<EmailWithRecipients>)> {
        Ok((0, vec![]))
    }
}

#[test]
fn send_email_form_excludes_recent_recipients() {
    let now = Utc::now().naive_utc();
    let names = HashMap::from([
        ("recent@example.com".to_string(), "Recent".to_string()),
        ("stale@example.com".to_string(), "Stale".to_string()),
    ]);
    let history = vec![
        HistoryEntry {
            email_id: 1,
            address: "recent@example.com".to_string(),
            is_sent: true,
            sent_at: now - Duration::days(1),
            updated_at: now,
        },
        HistoryEntry {
            email_id: 2,
            address: "stale@example.com".to_string(),
            is_sent: true,
            sent_at: now - Duration::days(10),
            updated_at: now,
        },
    ];
    let repo = CooldownRepository::new(1, names, history);

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

    let email = form.to_new_email(1, &repo).unwrap();

    assert_eq!(email.recipients.len(), 1);
    assert_eq!(
        email.recipients[0].address,
        RecipientEmail::new("stale@example.com").unwrap()
    );
}
