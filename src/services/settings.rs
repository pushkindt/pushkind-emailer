//! Business logic for application settings and history.
use crate::domain::hub::NewHub;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use pushkind_common::services::errors::{ServiceError, ServiceResult};
use validator::Validate;

use crate::dto::settings::{ExportedHistory, HistoryData, SettingsOverviewData, UnsubscribedData};

use crate::domain::recipient::CSVExportRecipient;
use crate::forms::settings::SaveHubForm;
use crate::models::config::ServerConfig;
use crate::repository::{EmailRecipientReader, HubReader, HubWriter, RecipientReader};

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
        None => repo.create_hub(&NewHub::try_new(user.hub_id).map_err(|err| {
            log::error!("Invalid hub id: {err}");
            ServiceError::Internal
        })?)?,
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

    form.validate()
        .map_err(|err| ServiceError::Form(err.to_string()))?;

    let hub = match repo.get_hub_by_id(user.hub_id)? {
        Some(hub) => hub,
        None => repo.create_hub(&NewHub::try_new(user.hub_id).map_err(|err| {
            log::error!("Invalid hub id: {err}");
            ServiceError::Internal
        })?)?,
    };

    let updates = form
        .try_into_update_hub()
        .map_err(|err| ServiceError::Form(err.to_string()))?;
    repo.update_hub(hub.id.get(), &updates)?;
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
