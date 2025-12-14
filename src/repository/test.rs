use std::collections::HashMap;

use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::email::{EmailRecipient, EmailWithRecipients};
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
        let recipient = Recipient::try_new(
            id,
            "Test",
            "test@test.test",
            hub_id,
            HashMap::new(),
            None,
            None,
            None,
            vec![],
        )
        .map_err(|err| RepositoryError::ValidationError(err.to_string()))?;
        Ok(Some(RecipientWithGroups {
            recipient,
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
    fn list_recent_recipients(
        &self,
        _hub_id: i32,
        _number_of_days: Option<i64>,
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
