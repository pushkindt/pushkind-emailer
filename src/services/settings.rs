use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::hub::{Hub, NewHub};
use pushkind_common::routes::check_role;
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

/// Loads the hub configuration, creating it if necessary.
pub fn load_settings_overview<R>(
    repo: &R,
    user: &AuthenticatedUser,
) -> ServiceResult<SettingsOverviewData>
where
    R: HubReader + HubWriter,
{
    ensure_admin(user)?;

    let hub = match repo.get_hub_by_id(user.hub_id)? {
        Some(hub) => hub,
        None => repo.create_hub(&NewHub::new(user.hub_id))?,
    };

    Ok(SettingsOverviewData { hub })
}

/// Lists unsubscribed recipients for the hub.
pub fn load_unsubscribed<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<UnsubscribedData>
where
    R: RecipientReader,
{
    ensure_emailer(user)?;

    let unsubscribed = repo.list_unsubscribed_recipients(user.hub_id)?;
    Ok(UnsubscribedData { unsubscribed })
}

/// Lists email history recipients for the hub.
pub fn load_history<R>(
    repo: &R,
    user: &AuthenticatedUser,
    server_config: &ServerConfig,
) -> ServiceResult<HistoryData>
where
    R: EmailRecipientReader,
{
    ensure_emailer(user)?;

    let history = repo.list_recent_recipients(user.hub_id, None)?;
    Ok(HistoryData {
        history,
        crm_service_url: server_config.crm_service_url.clone(),
    })
}

/// Exports the email recipient history as CSV.
pub fn export_history<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<ExportedHistory>
where
    R: EmailRecipientReader,
{
    ensure_emailer(user)?;

    let history = repo.list_recent_recipients(user.hub_id, None)?;

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
pub fn save_hub<R>(repo: &R, user: &AuthenticatedUser, form: SaveHubForm) -> ServiceResult<()>
where
    R: HubReader + HubWriter,
{
    ensure_admin(user)?;

    let hub = match repo.get_hub_by_id(user.hub_id)? {
        Some(hub) => hub,
        None => repo.create_hub(&NewHub::new(user.hub_id))?,
    };

    repo.update_hub(hub.id, &form.into())?;
    Ok(())
}

fn ensure_admin(user: &AuthenticatedUser) -> ServiceResult<()> {
    if check_role("admin", &user.roles) {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}

fn ensure_emailer(user: &AuthenticatedUser) -> ServiceResult<()> {
    if check_role("emailer", &user.roles) {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}
