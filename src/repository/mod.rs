use crate::domain::email::{
    EmailRecipient, EmailWithRecipients, NewEmail, UpdateEmail, UpdateEmailRecipient,
};
use crate::domain::group::{Group, GroupWithRecipients, NewGroup};
use crate::domain::hub::{Hub, NewHub, UpdateHub};
use crate::domain::recipient::{NewRecipient, Recipient, RecipientWithGroups, UpdateRecipient};
use crate::repository::errors::RepositoryResult;

pub mod email;
pub mod errors;
pub mod group;
pub mod hub;
pub mod recipient;

pub trait EmailReader {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<EmailWithRecipients>>;
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<EmailWithRecipients>>;
    fn get_recipient(&self, id: i32) -> RepositoryResult<Option<EmailRecipient>>;
}
pub trait EmailWriter {
    fn create(&self, email: &NewEmail) -> RepositoryResult<EmailWithRecipients>;
    fn update(&self, email_id: i32, updates: &UpdateEmail)
    -> RepositoryResult<EmailWithRecipients>;
    fn update_recipient(
        &self,
        recipient_id: i32,
        updates: &UpdateEmailRecipient,
    ) -> RepositoryResult<EmailWithRecipients>;
    fn delete(&self, id: i32) -> RepositoryResult<()>;
}

pub trait HubReader {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Hub>>;
    fn list(&self) -> RepositoryResult<Vec<Hub>>;
}

pub trait HubWriter {
    fn create(&self, hub: &NewHub) -> RepositoryResult<Hub>;
    fn update(&self, id: i32, hub: &UpdateHub) -> RepositoryResult<Hub>;
}

pub trait RecipientReader {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<RecipientWithGroups>>;
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Recipient>>;
    fn list_custom_fields(&self, hub_id: i32) -> RepositoryResult<Vec<String>>;
}
pub trait RecipientWriter {
    fn create(&self, recipient: &[NewRecipient]) -> RepositoryResult<usize>;
    fn update(&self, id: i32, recipient: &UpdateRecipient) -> RepositoryResult<Recipient>;
    fn delete(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all(&self, hub_id: i32) -> RepositoryResult<()>;
}

pub trait GroupReader {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Group>>;
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<GroupWithRecipients>>;
}
pub trait GroupWriter {
    fn create(&self, group: &NewGroup) -> RepositoryResult<Group>;
    fn delete(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all(&self, hub_id: i32) -> RepositoryResult<()>;
    fn assign_recipient(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()>;
    fn unassign_recipient(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()>;
}
