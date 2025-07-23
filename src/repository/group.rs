use diesel::prelude::*;
use pushkind_common::db::DbPool;

use crate::domain::group::{Group as DomainGroup, GroupWithRecipients, NewGroup as DomainNewGroup};
use crate::models::group::{Group as DbGroup, GroupRecipient, NewGroup as DbNewGroup};
use crate::models::recipient::Recipient as DbRecipient;
use crate::repository::errors::RepositoryResult;
use crate::repository::{GroupReader, GroupWriter};

/// Diesel implementation of [`GroupRepository`].
pub struct DieselGroupRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselGroupRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl GroupReader for DieselGroupRepository<'_> {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<GroupWithRecipients>> {
        use crate::schema::{groups, groups_recipients, recipients};
        let mut conn = self.pool.get()?;

        // Fetch groups for hub
        let db_groups: Vec<DbGroup> = groups::table
            .filter(groups::hub_id.eq(hub_id))
            .select(DbGroup::as_select())
            .load(&mut conn)?;

        if db_groups.is_empty() {
            return Ok(Vec::new());
        }

        // Fetch recipients for groups
        let group_recipients: Vec<(GroupRecipient, DbRecipient)> = groups_recipients::table
            .filter(groups_recipients::group_id.eq_any(db_groups.iter().map(|g| g.id)))
            .inner_join(recipients::table)
            .select((GroupRecipient::as_select(), DbRecipient::as_select()))
            .load(&mut conn)?;

        use std::collections::HashMap;
        let mut map: HashMap<i32, Vec<DbRecipient>> = HashMap::new();
        for (gr, rec) in group_recipients {
            map.entry(gr.group_id).or_default().push(rec);
        }

        Ok(db_groups
            .into_iter()
            .map(|g| {
                let recipients = map
                    .remove(&g.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| r.into())
                    .collect();
                GroupWithRecipients {
                    group: DomainGroup::from(g),
                    recipients,
                }
            })
            .collect())
    }
}

impl GroupWriter for DieselGroupRepository<'_> {
    fn create(&self, group: &DomainNewGroup) -> RepositoryResult<DomainGroup> {
        use crate::schema::groups;
        let mut conn = self.pool.get()?;
        let db_new = DbNewGroup {
            name: group.name,
            hub_id: group.hub_id,
        };
        let inserted = diesel::insert_into(groups::table)
            .values(&db_new)
            .get_result::<DbGroup>(&mut conn)?;
        Ok(inserted.into())
    }

    fn delete(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::groups;
        let mut conn = self.pool.get()?;
        diesel::delete(groups::table.filter(groups::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }

    fn assign_recipient(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()> {
        use crate::schema::groups_recipients;
        let mut conn = self.pool.get()?;
        let new = GroupRecipient {
            group_id,
            recipient_id,
        };
        diesel::insert_into(groups_recipients::table)
            .values(&new)
            .execute(&mut conn)?;
        Ok(())
    }

    fn unassign_recipient(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()> {
        use crate::schema::groups_recipients;

        let mut conn = self.pool.get()?;

        diesel::delete(
            groups_recipients::table.filter(
                groups_recipients::recipient_id
                    .eq(recipient_id)
                    .and(groups_recipients::group_id.eq(group_id)),
            ),
        )
        .execute(&mut conn)?;

        Ok(())
    }
}
