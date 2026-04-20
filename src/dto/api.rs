//! DTOs used by React-owned API endpoints.

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RecipientOptionDto {
    pub id: String,
    pub text: String,
    pub fields: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct RecipientAssignmentOptionDto {
    pub id: i32,
    pub text: String,
    pub fields: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct GroupOptionDto {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct EmailPreviewDto {
    pub id: i32,
    pub created_at: String,
    pub subject: Option<String>,
    pub message_html: String,
    pub message_preview: String,
    pub is_sent: bool,
    pub num_sent: i32,
    pub num_opened: i32,
    pub num_replied: i32,
    pub recipient_count: usize,
    pub recipients: Vec<EmailPreviewRecipientDto>,
}

#[derive(Debug, Serialize)]
pub struct EmailPreviewRecipientDto {
    pub address: String,
    pub opened: bool,
    pub is_sent: bool,
    pub reply: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RetryEmailDto {
    pub id: i32,
    pub subject: Option<String>,
    pub message: String,
    pub recipient_count: usize,
    pub recipient_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedEmailListDto {
    pub items: Vec<EmailPreviewDto>,
    pub pages: Vec<Option<usize>>,
    pub page: usize,
}

#[derive(Debug, Serialize)]
pub struct EmailCollectionDto {
    pub retry_email: Option<RetryEmailDto>,
    pub recipients: Vec<RecipientOptionDto>,
    pub groups: Vec<GroupOptionDto>,
    pub emails: PaginatedEmailListDto,
    pub custom_fields: Vec<String>,
    pub crm_service_url: String,
    pub files_service_url: String,
}

#[derive(Debug, Serialize)]
pub struct RecipientListItemDto {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedRecipientListDto {
    pub items: Vec<RecipientListItemDto>,
    pub pages: Vec<Option<usize>>,
    pub page: usize,
}

#[derive(Debug, Serialize)]
pub struct RecipientCollectionDto {
    pub recipients: PaginatedRecipientListDto,
    pub search_query: Option<String>,
    pub crm_service_url: String,
}

#[derive(Debug, Serialize)]
pub struct RecipientFieldDto {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct RecipientDetailsDto {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub unsubscribed_at: Option<String>,
    pub group_ids: Vec<i32>,
    pub fields: Vec<RecipientFieldDto>,
}

#[derive(Debug, Serialize)]
pub struct RecipientModalDto {
    pub recipient: RecipientDetailsDto,
    pub groups: Vec<GroupOptionDto>,
}

#[derive(Debug, Serialize)]
pub struct GroupListItemDto {
    pub id: i32,
    pub name: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupCollectionDto {
    pub groups: Vec<GroupListItemDto>,
    pub custom_fields: Vec<String>,
    pub recipients: Vec<RecipientAssignmentOptionDto>,
}

#[derive(Debug, Serialize)]
pub struct GroupDetailsDto {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct GroupModalDto {
    pub group: GroupDetailsDto,
    pub recipients: Vec<RecipientAssignmentOptionDto>,
}

#[derive(Debug, Serialize)]
pub struct HubSettingsDto {
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub message: Option<String>,
    pub files_service_url: String,
}

#[derive(Debug, Serialize)]
pub struct UnsubscribedItemDto {
    pub email: String,
    pub reason: Option<String>,
    pub unsubscribed_at: String,
}

#[derive(Debug, Serialize)]
pub struct UnsubscribedRecipientCollectionDto {
    pub items: Vec<UnsubscribedItemDto>,
}

#[derive(Debug, Serialize)]
pub struct HistoryItemDto {
    pub address: String,
    pub name: String,
    pub updated_at: String,
    pub opened: bool,
    pub replied: bool,
}

#[derive(Debug, Serialize)]
pub struct EmailHistoryCollectionDto {
    pub items: Vec<HistoryItemDto>,
    pub crm_service_url: String,
}
