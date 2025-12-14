use crate::domain::email::EmailWithRecipients;
use pushkind_common::pagination::Paginated;

use crate::domain::group::Group;
use crate::domain::recipient::Recipient;

/// Data required to render the main index page.
pub struct IndexPageData {
    pub retry_email: Option<EmailWithRecipients>,
    pub recipients: Vec<Recipient>,
    pub groups: Vec<Group>,
    pub emails: Paginated<EmailWithRecipients>,
    pub custom_fields: Vec<String>,
}

/// Result of exporting recipients for a specific email.
pub struct ExportedEmailRecipients {
    pub filename: String,
    pub bytes: Vec<u8>,
}
