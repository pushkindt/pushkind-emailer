use diesel::prelude::*;

use crate::domain::group::{Group as DomainGroup, NewGroup as DomainNewGroup};
use crate::models::hub::Hub;
use crate::models::recipient::Recipient;

#[derive(Queryable, Selectable, Identifiable, Associations, Clone, QueryableByName)]
#[diesel(table_name = crate::schema::groups)]
#[diesel(belongs_to(Hub, foreign_key = hub_id))]
#[diesel(foreign_derive)]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub hub_id: i32,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::groups)]
pub struct NewGroup<'a> {
    pub name: &'a str,
    pub hub_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Insertable)]
#[diesel(table_name = crate::schema::groups_recipients)]
#[diesel(belongs_to(Recipient, foreign_key = recipient_id))]
#[diesel(belongs_to(Group, foreign_key = group_id))]
#[diesel(primary_key(group_id, recipient_id))]
pub struct GroupRecipient {
    pub group_id: i32,
    pub recipient_id: i32,
}

impl From<Group> for DomainGroup {
    fn from(value: Group) -> Self {
        Self {
            id: value.id,
            name: value.name,
            hub_id: value.hub_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainNewGroup> for NewGroup<'a> {
    fn from(value: &'a DomainNewGroup) -> Self {
        Self {
            name: value.name.as_str(),
            hub_id: value.hub_id,
        }
    }
}
