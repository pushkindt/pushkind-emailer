use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::email::{
    EmailRecipient, EmailWithRecipients, NewEmail, UpdateEmail, UpdateEmailRecipient,
};
use crate::domain::group::{Group, GroupWithRecipients, NewGroup};
use crate::domain::hub::{Hub, NewHub, UpdateHub};
use crate::domain::recipient::{NewRecipient, Recipient, RecipientWithGroups, UpdateRecipient};

pub mod email;
pub mod group;
pub mod hub;
pub mod recipient;

#[derive(Clone)]
pub struct DieselRepository {
    pool: DbPool, // r2d2::Pool is cheap to clone
}

impl DieselRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

pub trait EmailReader {
    fn get_email_by_id(&self, id: i32) -> RepositoryResult<Option<EmailWithRecipients>>;
    fn list_emails(&self, hub_id: i32) -> RepositoryResult<Vec<EmailWithRecipients>>;
    fn list_emails_not_replied_recipients(
        &self,
        hub_id: i32,
    ) -> RepositoryResult<Vec<EmailRecipient>>;
    fn get_recipient(&self, id: i32) -> RepositoryResult<Option<EmailRecipient>>;
}
pub trait EmailWriter {
    fn create_email(&self, email: &NewEmail) -> RepositoryResult<EmailWithRecipients>;
    fn update_email(
        &self,
        email_id: i32,
        updates: &UpdateEmail,
    ) -> RepositoryResult<EmailWithRecipients>;
    fn update_recipient(
        &self,
        recipient_id: i32,
        updates: &UpdateEmailRecipient,
    ) -> RepositoryResult<EmailWithRecipients>;
    fn delete_email(&self, id: i32) -> RepositoryResult<()>;
}

pub trait HubReader {
    fn get_hub_by_id(&self, id: i32) -> RepositoryResult<Option<Hub>>;
    fn list_hubs(&self) -> RepositoryResult<Vec<Hub>>;
}

pub trait HubWriter {
    fn create_hub(&self, hub: &NewHub) -> RepositoryResult<Hub>;
    fn update_hub(&self, id: i32, hub: &UpdateHub) -> RepositoryResult<Hub>;
}

pub trait RecipientReader {
    fn get_recipient_by_id(&self, id: i32) -> RepositoryResult<Option<RecipientWithGroups>>;
    fn list_recipients(&self, hub_id: i32) -> RepositoryResult<Vec<Recipient>>;
    fn list_custom_fields(&self, hub_id: i32) -> RepositoryResult<Vec<String>>;
}
pub trait RecipientWriter {
    fn create_recipients(&self, recipient: &[NewRecipient]) -> RepositoryResult<usize>;
    fn update_recipient(&self, id: i32, recipient: &UpdateRecipient)
    -> RepositoryResult<Recipient>;
    fn delete_recipient(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all_recipients(&self, hub_id: i32) -> RepositoryResult<()>;
}

pub trait GroupReader {
    fn list_groups(&self, hub_id: i32) -> RepositoryResult<Vec<Group>>;
    fn get_group_by_id(&self, id: i32) -> RepositoryResult<Option<GroupWithRecipients>>;
}
pub trait GroupWriter {
    fn create_group(&self, group: &NewGroup) -> RepositoryResult<Group>;
    fn delete_group(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all_groups(&self, hub_id: i32) -> RepositoryResult<()>;
    fn assign_recipient_to_group(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()>;
    fn unassign_recipient_to_group(&self, group_id: i32, recipient_id: i32)
    -> RepositoryResult<()>;
}
