//! Repository layer for database I/O, queries, and persistence mapping.
use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::email::{EmailRecipient, EmailWithRecipients, UpdateEmailRecipient};
use crate::domain::hub::{Hub, NewHub, UpdateHub};

use crate::domain::group::{Group, GroupWithRecipients, NewGroup};
use crate::domain::recipient::{
    NewRecipient, Recipient, RecipientWithGroups, Unsubscribe, UpdateRecipient,
};
use crate::domain::types::{EmailId, GroupId, HubId, RecipientEmail, RecipientId};

mod helpers;

pub mod email;
pub mod group;
pub mod hub;
#[cfg(test)]
pub mod mock;
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

/// Query parameters used when listing or searching emails.
#[derive(Debug, Clone)]
pub struct EmailListQuery {
    /// Filter by hub identifier.
    pub hub_id: HubId,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl EmailListQuery {
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

/// Query parameters used when listing or searching recipients.
#[derive(Debug, Clone)]
pub struct RecipientListQuery {
    /// Filter by hub identifier.
    pub hub_id: HubId,
    /// Filter by group identifier.
    pub group_ids: Option<Vec<GroupId>>,
    /// Filter by email address.
    pub emails: Option<Vec<RecipientEmail>>,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl RecipientListQuery {
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            group_ids: None,
            emails: None,
            search: None,
            pagination: None,
        }
    }

    pub fn group_ids(mut self, group_ids: Vec<GroupId>) -> Self {
        self.group_ids = Some(group_ids);
        self
    }

    pub fn emails(mut self, emails: Vec<RecipientEmail>) -> Self {
        self.emails = Some(emails);
        self
    }

    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

/// Query parameters used when listing or searching groups.
#[derive(Debug, Clone)]
pub struct GroupListQuery {
    /// Filter by hub identifier.
    pub hub_id: HubId,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl GroupListQuery {
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

pub trait EmailReader {
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
        // Only include recipients whose most recent email was sent strictly
        // after `number_of_days` ago. `None` skips filtering.
        number_of_days: Option<i64>,
    ) -> RepositoryResult<Vec<EmailRecipient>>;
}
pub trait EmailWriter {
    fn update_email_recipient(
        &self,
        recipient_id: RecipientId,
        updates: &UpdateEmailRecipient,
    ) -> RepositoryResult<EmailWithRecipients>;
    fn delete_email(&self, id: EmailId, hub_id: HubId) -> RepositoryResult<()>;
}

pub trait HubReader {
    fn get_hub_by_id(&self, id: HubId) -> RepositoryResult<Option<Hub>>;
}

pub trait HubWriter {
    fn create_hub(&self, hub: &NewHub) -> RepositoryResult<Hub>;
    fn update_hub(&self, id: HubId, hub: &UpdateHub) -> RepositoryResult<Hub>;
}

pub trait RecipientReader {
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
pub trait RecipientWriter {
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

pub trait GroupReader {
    fn list_groups(&self, query: GroupListQuery) -> RepositoryResult<(usize, Vec<Group>)>;
    fn get_group_by_id(
        &self,
        id: GroupId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<GroupWithRecipients>>;
}
pub trait GroupWriter {
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
