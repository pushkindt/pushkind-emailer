use pushkind_common::domain::emailer::email::EmailRecipient;
use pushkind_common::domain::emailer::hub::Hub;

use crate::domain::recipient::Unsubscribe;

/// Data required to render the settings overview page.
pub struct SettingsOverviewData {
    pub hub: Hub,
}

/// Data required to render the unsubscribed recipients page.
pub struct UnsubscribedData {
    pub unsubscribed: Vec<Unsubscribe>,
}

/// Data required to render the history page.
pub struct HistoryData {
    pub history: Vec<EmailRecipient>,
    pub crm_service_url: String,
}

/// Result of exporting recipient history as CSV.
pub struct ExportedHistory {
    pub bytes: Vec<u8>,
}
