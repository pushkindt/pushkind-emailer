//! Mock repository implementations for isolating services in tests.

use mockall::mock;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::email::{EmailRecipient, EmailWithRecipients, UpdateEmailRecipient};
use crate::domain::group::{Group, GroupWithRecipients, NewGroup};
use crate::domain::hub::{Hub, NewHub, UpdateHub};
use crate::domain::recipient::{
    NewRecipient, Recipient, RecipientWithGroups, Unsubscribe, UpdateRecipient,
};
use crate::domain::types::{EmailId, EmailRecipientId, GroupId, HubId, RecipientId};
use crate::repository::{
    EmailListQuery, EmailReader, EmailWriter, GroupListQuery, GroupReader, GroupWriter, HubReader,
    HubWriter, RecipientListQuery, RecipientReader, RecipientWriter,
};

mock! {
    pub Repository {}

    impl EmailReader for Repository {
        fn get_email_by_id(
            &self,
            id: EmailId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<EmailWithRecipients>>;
        fn list_emails(
            &self,
            query: EmailListQuery,
        ) -> RepositoryResult<(usize, Vec<EmailWithRecipients>)>;
        fn list_recent_email_recipients(
            &self,
            hub_id: HubId,
            number_of_days: Option<i64>,
        ) -> RepositoryResult<Vec<EmailRecipient>>;
    }

    impl EmailWriter for Repository {
        fn update_email_recipient(
            &self,
            recipient_id: EmailRecipientId,
            updates: &UpdateEmailRecipient,
        ) -> RepositoryResult<EmailWithRecipients>;
        fn delete_email(&self, id: EmailId, hub_id: HubId) -> RepositoryResult<()>;
    }

    impl GroupReader for Repository {
        fn list_groups(&self, query: GroupListQuery) -> RepositoryResult<(usize, Vec<Group>)>;
        fn get_group_by_id(
            &self,
            id: GroupId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<GroupWithRecipients>>;
    }

    impl GroupWriter for Repository {
        fn create_group(&self, group: &NewGroup) -> RepositoryResult<Group>;
        fn delete_group(&self, id: GroupId, hub_id: HubId) -> RepositoryResult<()>;
        fn delete_all_groups(&self, hub_id: HubId) -> RepositoryResult<()>;
        fn assign_recipients_to_group(
            &self,
            group_id: GroupId,
            recipients: Vec<RecipientId>,
            hub_id: HubId,
        ) -> RepositoryResult<()>;
    }

    impl HubReader for Repository {
        fn get_hub_by_id(&self, id: HubId) -> RepositoryResult<Option<Hub>>;
    }

    impl HubWriter for Repository {
        fn create_hub(&self, hub: &NewHub) -> RepositoryResult<Hub>;
        fn update_hub(&self, id: HubId, hub: &UpdateHub) -> RepositoryResult<Hub>;
    }

    impl RecipientReader for Repository {
        fn get_recipient_by_id(
            &self,
            id: RecipientId,
            hub_id: HubId,
        ) -> RepositoryResult<Option<RecipientWithGroups>>;
        fn list_recipients(
            &self,
            query: RecipientListQuery,
        ) -> RepositoryResult<(usize, Vec<Recipient>)>;
        fn list_custom_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<String>>;
        fn list_unsubscribed_recipients(&self, hub_id: HubId) -> RepositoryResult<Vec<Unsubscribe>>;
    }

    impl RecipientWriter for Repository {
        fn create_recipients(&self, recipient: &[NewRecipient]) -> RepositoryResult<usize>;
        fn update_recipient(
            &self,
            id: RecipientId,
            hub_id: HubId,
            recipient: &UpdateRecipient,
        ) -> RepositoryResult<Recipient>;
        fn delete_recipient(&self, id: RecipientId, hub_id: HubId) -> RepositoryResult<()>;
        fn delete_all_recipients(&self, hub_id: HubId) -> RepositoryResult<()>;
    }
}
