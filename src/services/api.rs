//! Service adaptors serving Emailer API data.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use serde::Deserialize;

use crate::domain::email::EmailWithRecipients;
use crate::domain::group::Group;
use crate::domain::recipient::{Recipient, RecipientWithGroups};
use crate::dto::api::{
    EmailCollectionDto, EmailHistoryCollectionDto, EmailPreviewDto, EmailPreviewRecipientDto,
    GroupCollectionDto, GroupDetailsDto, GroupListItemDto, GroupModalDto, GroupOptionDto,
    HistoryItemDto, HubSettingsDto, IamDto, NavigationItemDto, NoAccessPageDto,
    PaginatedEmailListDto, PaginatedRecipientListDto, RecipientAssignmentOptionDto,
    RecipientCollectionDto, RecipientDetailsDto, RecipientFieldDto, RecipientListItemDto,
    RecipientModalDto, RecipientOptionDto, RetryEmailDto, UnsubscribedItemDto,
    UnsubscribedRecipientCollectionDto,
};
use crate::dto::main::IndexQueryParams;
use crate::dto::recipients::RecipientsQueryParams;
use crate::models::config::AppConfig;
use crate::repository::{EmailReader, GroupReader, HubReader, HubWriter, RecipientReader};
use crate::services::{ServiceResult, ensure_admin, ensure_emailer};

#[derive(Debug, Deserialize)]
struct SerializedPaginatedEmails {
    items: Vec<SerializedEmailWithRecipients>,
    pages: Vec<Option<usize>>,
    page: usize,
}

#[derive(Debug, Deserialize)]
struct SerializedEmailWithRecipients {
    email: SerializedEmail,
    recipients: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SerializedEmail {
    id: i32,
    message: String,
    created_at: String,
    is_sent: bool,
    subject: Option<String>,
    num_sent: i32,
    num_opened: i32,
    num_replied: i32,
}

#[derive(Debug, Deserialize)]
struct SerializedPaginatedRecipients {
    items: Vec<SerializedRecipient>,
    pages: Vec<Option<usize>>,
    page: usize,
}

#[derive(Debug, Deserialize)]
struct SerializedRecipient {
    id: i32,
    name: String,
    email: String,
    fields: std::collections::BTreeMap<String, String>,
}

fn recipient_option(recipient: &Recipient) -> RecipientOptionDto {
    RecipientOptionDto {
        id: recipient.email.as_str().to_owned(),
        text: format!("{} ({})", recipient.name, recipient.email),
        fields: recipient.fields.clone(),
    }
}

fn recipient_assignment_option(recipient: &Recipient) -> RecipientAssignmentOptionDto {
    RecipientAssignmentOptionDto {
        id: recipient.id.get(),
        text: format!("{} ({})", recipient.name, recipient.email),
        fields: recipient.fields.clone(),
    }
}

fn group_option(group: &Group) -> GroupOptionDto {
    GroupOptionDto {
        id: group.id.get(),
        name: group.name.to_string(),
    }
}

fn group_list_item(group: &Group) -> GroupListItemDto {
    GroupListItemDto {
        id: group.id.get(),
        name: group.name.to_string(),
        created_at: group
            .created_at
            .map(|created_at| created_at.format("%Y-%m-%d %H:%M").to_string()),
    }
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

fn format_datetime_without_fractional_seconds(value: &str) -> String {
    let without_fraction = value.split('.').next().unwrap_or(value);
    without_fraction.replace('T', " ")
}

fn recipient_list_item(recipient: &SerializedRecipient) -> RecipientListItemDto {
    RecipientListItemDto {
        id: recipient.id,
        name: recipient.name.clone(),
        email: recipient.email.clone(),
        fields: recipient.fields.clone(),
    }
}

fn recipient_details(recipient_with_groups: &RecipientWithGroups) -> RecipientDetailsDto {
    RecipientDetailsDto {
        id: recipient_with_groups.recipient.id.get(),
        name: recipient_with_groups.recipient.name.to_string(),
        email: recipient_with_groups.recipient.email.to_string(),
        unsubscribed_at: recipient_with_groups
            .recipient
            .unsubscribed_at
            .map(|date| date.format("%Y-%m-%d %H:%M").to_string()),
        group_ids: recipient_with_groups
            .recipient
            .groups
            .iter()
            .map(|group_id| group_id.get())
            .collect(),
        fields: recipient_with_groups
            .recipient
            .fields
            .iter()
            .map(|(name, value)| RecipientFieldDto {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
    }
}

fn email_preview(email_with_recipients: &SerializedEmailWithRecipients) -> EmailPreviewDto {
    let email = &email_with_recipients.email;
    let message = email.message.as_str();
    let message_preview = truncate_preview(message, 160);

    EmailPreviewDto {
        id: email.id,
        created_at: format_datetime_without_fractional_seconds(&email.created_at),
        subject: email.subject.clone(),
        message_html: email.message.clone(),
        message_preview,
        is_sent: email.is_sent,
        num_sent: email.num_sent,
        num_opened: email.num_opened,
        num_replied: email.num_replied,
        recipient_count: email_with_recipients.recipients.len(),
        recipients: email_with_recipients
            .recipients
            .iter()
            .filter_map(|recipient| {
                recipient
                    .as_object()
                    .map(|record| EmailPreviewRecipientDto {
                        address: record
                            .get("address")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        opened: record
                            .get("opened")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                        is_sent: record
                            .get("is_sent")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                        reply: record
                            .get("reply")
                            .and_then(|value| value.as_str())
                            .map(ToOwned::to_owned),
                    })
            })
            .collect(),
    }
}

fn retry_email_dto(email_with_recipients: &EmailWithRecipients) -> RetryEmailDto {
    RetryEmailDto {
        id: email_with_recipients.email.id.get(),
        subject: email_with_recipients
            .email
            .subject
            .as_ref()
            .map(ToString::to_string),
        message: email_with_recipients.email.message.as_str().to_owned(),
        recipient_count: email_with_recipients.recipients.len(),
        recipient_ids: email_with_recipients
            .recipients
            .iter()
            .map(|recipient| recipient.address.as_str().to_owned())
            .collect(),
    }
}

/// Returns typed shell data for React-owned Emailer pages.
pub fn get_shell_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> ServiceResult<IamDto> {
    ensure_emailer(user)?;

    let mut local_menu_items = vec![
        NavigationItemDto {
            name: "Отписавшиеся",
            url: "/unsubscribed",
        },
        NavigationItemDto {
            name: "История",
            url: "/history",
        },
    ];

    if ensure_admin(user).is_ok() {
        local_menu_items.insert(
            0,
            NavigationItemDto {
                name: "Настройки",
                url: "/settings",
            },
        );
    }

    Ok(IamDto {
        current_user: user.into(),
        home_url: common_config.auth_service_url.clone(),
        navigation: vec![
            NavigationItemDto {
                name: "Сообщения",
                url: "/",
            },
            NavigationItemDto {
                name: "Получатели",
                url: "/recipients",
            },
            NavigationItemDto {
                name: "Группы",
                url: "/groups",
            },
        ],
        local_menu_items,
    })
}

/// Returns typed email collection data for the Emailer index page.
pub fn get_emails_data<R>(
    params: IndexQueryParams,
    user: &AuthenticatedUser,
    repo: &R,
    app_config: &AppConfig,
) -> ServiceResult<EmailCollectionDto>
where
    R: EmailReader + RecipientReader + GroupReader,
{
    let data = crate::services::main::load_index_page(params, user, repo)?;
    let paginated_emails: SerializedPaginatedEmails = serde_json::from_value(
        serde_json::to_value(data.emails)
            .map_err(|_| pushkind_common::services::errors::ServiceError::Internal)?,
    )
    .map_err(|_| pushkind_common::services::errors::ServiceError::Internal)?;

    Ok(EmailCollectionDto {
        retry_email: data.retry_email.as_ref().map(retry_email_dto),
        recipients: data
            .recipients
            .iter()
            .filter(|recipient| recipient.unsubscribed_at.is_none())
            .map(recipient_option)
            .collect::<Vec<RecipientOptionDto>>(),
        groups: data
            .groups
            .iter()
            .map(group_option)
            .collect::<Vec<GroupOptionDto>>(),
        emails: PaginatedEmailListDto {
            items: paginated_emails
                .items
                .iter()
                .map(email_preview)
                .collect::<Vec<EmailPreviewDto>>(),
            pages: paginated_emails.pages,
            page: paginated_emails.page,
        },
        custom_fields: data.custom_fields,
        crm_service_url: app_config.crm_service_url.clone(),
    })
}

/// Returns typed recipient collection data.
pub fn get_recipients_data<R>(
    params: RecipientsQueryParams,
    user: &AuthenticatedUser,
    repo: &R,
    app_config: &AppConfig,
) -> ServiceResult<RecipientCollectionDto>
where
    R: RecipientReader,
{
    let data = crate::services::recipients::load_recipients_overview(params, user, repo)?;
    let paginated_recipients: SerializedPaginatedRecipients = serde_json::from_value(
        serde_json::to_value(data.recipients)
            .map_err(|_| pushkind_common::services::errors::ServiceError::Internal)?,
    )
    .map_err(|_| pushkind_common::services::errors::ServiceError::Internal)?;

    Ok(RecipientCollectionDto {
        recipients: PaginatedRecipientListDto {
            items: paginated_recipients
                .items
                .iter()
                .map(recipient_list_item)
                .collect(),
            pages: paginated_recipients.pages,
            page: paginated_recipients.page,
        },
        search_query: data.search_query,
        crm_service_url: app_config.crm_service_url.clone(),
    })
}

/// Returns typed modal data for a single recipient.
pub fn get_recipient_modal_data<R>(
    recipient_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<RecipientModalDto>
where
    R: RecipientReader + GroupReader,
{
    let data = crate::services::recipients::load_recipient_modal(recipient_id, user, repo)?;

    Ok(RecipientModalDto {
        recipient: recipient_details(&data.recipient),
        groups: data.groups.iter().map(group_option).collect(),
    })
}

/// Returns typed group collection data.
pub fn get_groups_data<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<GroupCollectionDto>
where
    R: GroupReader + RecipientReader,
{
    let data = crate::services::groups::load_groups_overview(user, repo)?;

    Ok(GroupCollectionDto {
        groups: data.groups.iter().map(group_list_item).collect(),
        custom_fields: data.custom_fields,
        recipients: data
            .recipients
            .iter()
            .map(recipient_assignment_option)
            .collect(),
    })
}

/// Returns typed modal data for a single group.
pub fn get_group_modal_data<R>(
    group_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<GroupModalDto>
where
    R: GroupReader,
{
    let data = crate::services::groups::load_group_modal(group_id, user, repo)?;

    Ok(GroupModalDto {
        group: GroupDetailsDto {
            id: data.group.id.get(),
            name: data.group.name.to_string(),
        },
        recipients: data
            .recipients
            .iter()
            .map(recipient_assignment_option)
            .collect(),
    })
}

/// Returns typed hub settings data.
pub fn get_hub_settings_data<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<HubSettingsDto>
where
    R: HubReader + HubWriter,
{
    let data = crate::services::settings::load_settings_overview(user, repo)?;

    Ok(HubSettingsDto {
        login: data.hub.login.as_ref().map(ToString::to_string),
        password: data
            .hub
            .password
            .as_ref()
            .map(|value| value.as_str().to_owned()),
        sender: data.hub.sender.as_ref().map(ToString::to_string),
        smtp_server: data.hub.smtp_server.as_ref().map(ToString::to_string),
        smtp_port: data.hub.smtp_port.as_ref().map(|value| value.get() as i32),
        imap_server: data.hub.imap_server.as_ref().map(ToString::to_string),
        imap_port: data.hub.imap_port.as_ref().map(|value| value.get() as i32),
        message: data.hub.email_template.as_ref().map(ToString::to_string),
    })
}

/// Returns typed unsubscribed recipient collection data.
pub fn get_unsubscribed_recipients_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<UnsubscribedRecipientCollectionDto>
where
    R: RecipientReader,
{
    let data = crate::services::settings::load_unsubscribed(user, repo)?;

    Ok(UnsubscribedRecipientCollectionDto {
        items: data
            .unsubscribed
            .iter()
            .map(|item| UnsubscribedItemDto {
                email: item.email.to_string(),
                reason: item.reason.as_ref().map(ToString::to_string),
                unsubscribed_at: item.unsubscribed_at.format("%Y-%m-%d %H:%M").to_string(),
            })
            .collect(),
    })
}

/// Returns typed email history collection data.
pub fn get_email_history_data<R>(
    user: &AuthenticatedUser,
    repo: &R,
    app_config: &AppConfig,
) -> ServiceResult<EmailHistoryCollectionDto>
where
    R: EmailReader,
{
    let data = crate::services::settings::load_history(user, repo, app_config)?;

    Ok(EmailHistoryCollectionDto {
        items: data
            .history
            .iter()
            .map(|item| HistoryItemDto {
                address: item.address.to_string(),
                name: item.name.to_string(),
                updated_at: item.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                opened: item.opened,
                replied: item.reply.is_some(),
            })
            .collect(),
        crm_service_url: data.crm_service_url,
    })
}

/// Returns typed page data for the React-owned no-access page.
pub fn get_no_access_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> NoAccessPageDto {
    NoAccessPageDto {
        current_user: user.into(),
        home_url: common_config.auth_service_url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_datetime_without_fractional_seconds, truncate_preview};

    #[test]
    fn truncate_preview_keeps_utf8_boundaries() {
        let value = format!("{}б", "a".repeat(160));

        let preview = truncate_preview(&value, 160);

        assert_eq!(preview.chars().count(), 161);
        assert!(preview.ends_with('…'));
        assert_eq!(preview, format!("{}…", "a".repeat(160)));
    }

    #[test]
    fn truncate_preview_keeps_short_values_unchanged() {
        let value = "короткий текст";

        assert_eq!(truncate_preview(value, 160), value);
    }

    #[test]
    fn timestamp_format_drops_fractional_seconds() {
        let value = "2026-03-12T08:27:29.079633692";

        assert_eq!(
            format_datetime_without_fractional_seconds(value),
            "2026-03-12 08:27:29"
        );
    }
}
