//! DTOs used by recipient-related routes and templates.
use pushkind_common::pagination::Paginated;
use serde::Deserialize;

use crate::domain::group::Group;
use crate::domain::recipient::{Recipient, RecipientWithGroups};

#[derive(Deserialize)]
pub struct RecipientsQueryParams {
    pub q: Option<String>,
    pub page: Option<usize>,
}

/// Data required to render the recipients overview page.
pub struct RecipientsOverviewData {
    pub recipients: Paginated<Recipient>,
    pub search_query: Option<String>,
}

/// Data required to render the recipient modal dialog.
pub struct RecipientModalData {
    pub recipient: RecipientWithGroups,
    pub groups: Vec<Group>,
}
