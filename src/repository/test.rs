use std::collections::HashMap;

use pushkind_common::{
    domain::emailer::email::{EmailRecipient, EmailWithRecipients},
    repository::errors::RepositoryResult,
};

use crate::domain::recipient::{Recipient, RecipientWithGroups};
use crate::repository::{
    EmailListQuery, EmailReader, EmailRecipientReader, RecipientListQuery, RecipientReader,
};

#[derive(Clone)]
pub struct TestRepository;

impl RecipientReader for TestRepository {
    fn get_recipient_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<RecipientWithGroups>> {
        Ok(Some(RecipientWithGroups {
            recipient: Recipient {
                id,
                name: "Test".to_string(),
                email: "test@test.test".to_string(),
                hub_id,
                fields: HashMap::new(),
                created_at: None,
                updated_at: None,
                unsubscribed_at: None,
                groups: vec![],
            },
            groups: vec![],
        }))
    }
    fn list_custom_fields(&self, _hub_id: i32) -> RepositoryResult<Vec<String>> {
        Ok(vec![])
    }
    fn list_recipients(
        &self,
        _query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<Recipient>)> {
        Ok((0, vec![]))
    }
    fn search_recipients(
        &self,
        _query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<Recipient>)> {
        Ok((0, vec![]))
    }
    fn list_unsubscribed_recipients(
        &self,
        _hub_id: i32,
    ) -> RepositoryResult<Vec<crate::domain::recipient::Unsubscribe>> {
        Ok(vec![])
    }
}

impl EmailRecipientReader for TestRepository {
    fn list_recipients_grouped_by_address(
        &self,
        _hub_id: i32,
    ) -> RepositoryResult<Vec<EmailRecipient>> {
        Ok(vec![])
    }
}

impl EmailReader for TestRepository {
    fn get_email_by_id(
        &self,
        _id: i32,
        _hub_id: i32,
    ) -> RepositoryResult<Option<EmailWithRecipients>> {
        Ok(None)
    }

    fn list_emails(
        &self,
        _query: EmailListQuery,
    ) -> RepositoryResult<(usize, Vec<EmailWithRecipients>)> {
        Ok((0, vec![]))
    }
}
