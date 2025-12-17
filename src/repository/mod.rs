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

mod helpers;

pub mod email;
pub mod group;
pub mod hub;
pub mod recipient;
pub mod test;

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
    pub hub_id: i32,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl EmailListQuery {
    pub fn new(hub_id: i32) -> Self {
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
    pub hub_id: i32,
    /// Filter by group identifier.
    pub group_ids: Option<Vec<i32>>,
    /// Filter by email address.
    pub emails: Option<Vec<String>>,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl RecipientListQuery {
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            group_ids: None,
            emails: None,
            search: None,
            pagination: None,
        }
    }

    pub fn group_ids(mut self, group_ids: Vec<i32>) -> Self {
        self.group_ids = Some(group_ids);
        self
    }

    pub fn emails(mut self, emails: Vec<String>) -> Self {
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
    pub hub_id: i32,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl GroupListQuery {
    pub fn new(hub_id: i32) -> Self {
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
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<EmailWithRecipients>>;
    fn list_emails(
        &self,
        query: EmailListQuery,
    ) -> RepositoryResult<(usize, Vec<EmailWithRecipients>)>;
}
pub trait EmailWriter {
    fn update_recipient(
        &self,
        recipient_id: i32,
        updates: &UpdateEmailRecipient,
    ) -> RepositoryResult<EmailWithRecipients>;
    fn delete_email(&self, id: i32) -> RepositoryResult<()>;
}

pub trait EmailRecipientReader {
    /// Return recipients grouped by email address for the provided hub.
    ///
    /// When the same email address received multiple emails within the hub,
    /// the record belonging to the most recently created email is returned so
    /// that callers always get the latest snapshot of the recipient data.
    fn list_recent_recipients(
        &self,
        hub_id: i32,
        // Only include recipients whose most recent email was sent strictly
        // after `number_of_days` ago. `None` skips filtering.
        number_of_days: Option<i64>,
    ) -> RepositoryResult<Vec<EmailRecipient>>;
}

pub trait HubReader {
    fn get_hub_by_id(&self, id: i32) -> RepositoryResult<Option<Hub>>;
}

pub trait HubWriter {
    fn create_hub(&self, hub: &NewHub) -> RepositoryResult<Hub>;
    fn update_hub(&self, id: i32, hub: &UpdateHub) -> RepositoryResult<Hub>;
}

pub trait RecipientReader {
    fn get_recipient_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<RecipientWithGroups>>;
    fn list_recipients(
        &self,
        query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<Recipient>)>;
    fn list_custom_fields(&self, hub_id: i32) -> RepositoryResult<Vec<String>>;
    fn list_unsubscribed_recipients(&self, hub_id: i32) -> RepositoryResult<Vec<Unsubscribe>>;
}
pub trait RecipientWriter {
    fn create_recipients(&self, recipient: &[NewRecipient]) -> RepositoryResult<usize>;
    fn update_recipient(&self, id: i32, recipient: &UpdateRecipient)
    -> RepositoryResult<Recipient>;
    fn delete_recipient(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all_recipients(&self, hub_id: i32) -> RepositoryResult<()>;
}

pub trait GroupReader {
    fn list_groups(&self, query: GroupListQuery) -> RepositoryResult<(usize, Vec<Group>)>;
    fn get_group_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<GroupWithRecipients>>;
}
pub trait GroupWriter {
    fn create_group(&self, group: &NewGroup) -> RepositoryResult<Group>;
    fn delete_group(&self, id: i32) -> RepositoryResult<()>;
    fn delete_all_groups(&self, hub_id: i32) -> RepositoryResult<()>;
    fn assign_recipients_to_group(
        &self,
        group_id: i32,
        recipients: Vec<i32>,
    ) -> RepositoryResult<()>;
}
