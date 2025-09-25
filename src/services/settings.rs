use pushkind_common::domain::emailer::hub::{Hub, NewHub};
use pushkind_common::services::errors::{ServiceError, ServiceResult};

use pushkind_common::domain::emailer::email::EmailRecipient;

use crate::domain::recipient::{CSVExportRecipient, Unsubscribe};
use crate::forms::settings::SaveHubForm;
use crate::models::config::ServerConfig;
use crate::repository::{EmailRecipientReader, HubReader, HubWriter, RecipientReader};

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

/// Service encapsulating settings operations.
pub struct SettingsService<'a, R>
where
    R: HubReader + HubWriter + RecipientReader + EmailRecipientReader,
{
    repo: &'a R,
}

impl<'a, R> SettingsService<'a, R>
where
    R: HubReader + HubWriter + RecipientReader + EmailRecipientReader,
{
    pub fn new(repo: &'a R) -> Self {
        Self { repo }
    }

    /// Loads the hub configuration, creating it if necessary.
    pub fn load_overview(&self, hub_id: i32) -> ServiceResult<SettingsOverviewData> {
        let hub = match self.repo.get_hub_by_id(hub_id)? {
            Some(hub) => hub,
            None => self.repo.create_hub(&NewHub::new(hub_id))?,
        };

        Ok(SettingsOverviewData { hub })
    }

    /// Lists unsubscribed recipients for the hub.
    pub fn load_unsubscribed(&self, hub_id: i32) -> ServiceResult<UnsubscribedData> {
        let unsubscribed = self.repo.list_unsubscribed_recipients(hub_id)?;
        Ok(UnsubscribedData { unsubscribed })
    }

    /// Lists email history recipients for the hub.
    pub fn load_history(
        &self,
        hub_id: i32,
        server_config: &ServerConfig,
    ) -> ServiceResult<HistoryData> {
        let history = self.repo.list_recent_recipients(hub_id, None)?;
        Ok(HistoryData {
            history,
            crm_service_url: server_config.crm_service_url.clone(),
        })
    }

    /// Exports the email recipient history as CSV.
    pub fn export_history(&self, hub_id: i32) -> ServiceResult<ExportedHistory> {
        let history = self.repo.list_recent_recipients(hub_id, None)?;

        let mut writer = csv::Writer::from_writer(vec![]);
        for recipient in history {
            let recipient = CSVExportRecipient::from(recipient);
            writer.serialize(recipient).map_err(|err| {
                log::error!("Failed to serialize recipient: {err}");
                ServiceError::Internal
            })?;
        }

        let bytes = writer.into_inner().map_err(|err| {
            log::error!("Failed to finalize csv: {err}");
            ServiceError::Internal
        })?;

        Ok(ExportedHistory { bytes })
    }

    /// Persists the hub configuration changes.
    pub fn save_hub(&self, hub_id: i32, form: SaveHubForm) -> ServiceResult<()> {
        let hub = match self.repo.get_hub_by_id(hub_id)? {
            Some(hub) => hub,
            None => self.repo.create_hub(&NewHub::new(hub_id))?,
        };

        self.repo.update_hub(hub.id, &form.into())?;
        Ok(())
    }
}
