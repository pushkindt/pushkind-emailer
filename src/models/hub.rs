//! Diesel models backing hub persistence.
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::hub::{Hub as DomainHub, NewHub as DomainNewHub, UpdateHub as DomainUpdateHub};
use crate::domain::types::{
    EmailTemplate, HubId, HubLogin, HubPassword, HubSenderName, ImapPort, ImapServerHost, ImapUid,
    SmtpPort, SmtpServerHost, TypeConstraintError,
};

#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::hubs)]
pub struct Hub {
    pub id: i32,
    pub login: Option<String>,
    pub password: Option<String>,
    pub sender: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<String>,
    pub imap_port: Option<i32>,
    pub email_template: Option<String>,
    pub imap_last_uid: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::hubs)]
pub struct NewHub<'a> {
    pub id: i32,
    pub login: Option<&'a str>,
    pub password: Option<&'a str>,
    pub sender: Option<&'a str>,
    pub smtp_server: Option<&'a str>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<&'a str>,
    pub imap_port: Option<i32>,
    pub email_template: Option<&'a str>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::hubs)]
pub struct UpdateHub<'a> {
    pub login: Option<&'a str>,
    pub password: Option<&'a str>,
    pub sender: Option<&'a str>,
    pub smtp_server: Option<&'a str>,
    pub smtp_port: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub imap_server: Option<&'a str>,
    pub imap_port: Option<i32>,
    pub email_template: Option<&'a str>,
}

impl TryFrom<Hub> for DomainHub {
    type Error = TypeConstraintError;

    fn try_from(value: Hub) -> Result<Self, Self::Error> {
        Ok(Self {
            id: HubId::try_from(value.id)?,
            login: value.login.map(HubLogin::try_from).transpose()?,
            password: value.password.map(HubPassword::try_from).transpose()?,
            sender: value.sender.map(HubSenderName::try_from).transpose()?,
            smtp_server: value
                .smtp_server
                .map(SmtpServerHost::try_from)
                .transpose()?,
            smtp_port: value.smtp_port.map(SmtpPort::try_from).transpose()?,
            created_at: value.created_at,
            updated_at: value.updated_at,
            imap_server: value
                .imap_server
                .map(ImapServerHost::try_from)
                .transpose()?,
            imap_port: value.imap_port.map(ImapPort::try_from).transpose()?,
            email_template: value
                .email_template
                .map(EmailTemplate::try_from)
                .transpose()?,
            imap_last_uid: ImapUid::try_from(value.imap_last_uid)?,
        })
    }
}

impl<'a> From<&'a DomainNewHub> for NewHub<'a> {
    fn from(value: &'a DomainNewHub) -> Self {
        Self {
            id: value.id.get(),
            login: value.login.as_ref().map(|login| login.as_str()),
            password: value.password.as_ref().map(|password| password.as_str()),
            sender: value.sender.as_ref().map(|sender| sender.as_str()),
            smtp_server: value.smtp_server.as_ref().map(|host| host.as_str()),
            smtp_port: value.smtp_port.map(Into::into),
            created_at: value.created_at,
            updated_at: value.updated_at,
            imap_server: value.imap_server.as_ref().map(|host| host.as_str()),
            imap_port: value.imap_port.map(Into::into),
            email_template: value.email_template.as_ref().map(|tpl| tpl.as_str()),
        }
    }
}

impl<'a> From<&'a DomainUpdateHub> for UpdateHub<'a> {
    fn from(value: &'a DomainUpdateHub) -> Self {
        Self {
            login: value.login.as_ref().map(|login| login.as_str()),
            password: value.password.as_ref().map(|password| password.as_str()),
            sender: value.sender.as_ref().map(|sender| sender.as_str()),
            smtp_server: value.smtp_server.as_ref().map(|host| host.as_str()),
            smtp_port: value.smtp_port.map(Into::into),
            created_at: value.created_at,
            updated_at: value.updated_at,
            imap_server: value.imap_server.as_ref().map(|host| host.as_str()),
            imap_port: value.imap_port.map(Into::into),
            email_template: value.email_template.as_ref().map(|tpl| tpl.as_str()),
        }
    }
}
