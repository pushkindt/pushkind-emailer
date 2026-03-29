//! Business logic for application settings and history.
use crate::domain::hub::NewHub;
use crate::domain::types::HubId;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::dto::settings::{ExportedHistory, HistoryData, SettingsOverviewData, UnsubscribedData};
use crate::services::{ensure_admin, ensure_emailer};

use crate::domain::recipient::CSVExportRecipient;
use crate::forms::settings::SaveHubPayload;
use crate::models::config::AppConfig;
use crate::repository::{EmailReader, HubReader, HubWriter, RecipientReader};

/// Loads the hub configuration, creating it if necessary.
pub fn load_settings_overview<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<SettingsOverviewData>
where
    R: HubReader + HubWriter,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let hub = load_or_create_hub(hub_id, repo)?;

    Ok(SettingsOverviewData { hub })
}

/// Lists unsubscribed recipients for the hub.
pub fn load_unsubscribed<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<UnsubscribedData>
where
    R: RecipientReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let unsubscribed = repo.list_unsubscribed_recipients(hub_id)?;
    Ok(UnsubscribedData { unsubscribed })
}

/// Lists email history recipients for the hub.
pub fn load_history<R>(
    user: &AuthenticatedUser,
    repo: &R,
    app_config: &AppConfig,
) -> ServiceResult<HistoryData>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let history = repo.list_recent_email_recipients(hub_id, None)?;
    Ok(HistoryData {
        history,
        crm_service_url: app_config.crm_service_url.clone(),
    })
}

/// Exports the email recipient history as CSV.
pub fn export_history<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<ExportedHistory>
where
    R: EmailReader,
{
    ensure_emailer(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let history = repo.list_recent_email_recipients(hub_id, None)?;

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
pub fn save_hub<R>(payload: SaveHubPayload, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: HubReader + HubWriter,
{
    ensure_admin(user)?;

    let hub_id = HubId::new(user.hub_id)?;

    let hub = load_or_create_hub(hub_id, repo)?;

    let updates = payload.into_domain();
    repo.update_hub(hub.id, &updates)?;
    Ok(())
}

fn load_or_create_hub<R>(hub_id: HubId, repo: &R) -> ServiceResult<crate::domain::hub::Hub>
where
    R: HubReader + HubWriter,
{
    match repo.get_hub_by_id(hub_id)? {
        Some(hub) => Ok(hub),
        None => {
            let hub = NewHub::new(hub_id, None, None, None, None, None, None, None, None);
            Ok(repo.create_hub(&hub)?)
        }
    }
}
