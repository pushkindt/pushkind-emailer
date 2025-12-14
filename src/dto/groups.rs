//! DTOs used by group-related routes and templates.
use crate::domain::group::Group;
use crate::domain::recipient::Recipient;

/// Data required to render the groups overview page.
pub struct GroupsOverviewData {
    pub groups: Vec<Group>,
    pub custom_fields: Vec<String>,
    pub recipients: Vec<Recipient>,
}
