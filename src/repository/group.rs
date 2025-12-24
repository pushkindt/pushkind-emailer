//! Repository operations for recipient groups.
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
use std::collections::BTreeSet;

use super::helpers::{apply_pagination, hydrate_recipients};
use crate::domain::group::{Group as DomainGroup, GroupWithRecipients, NewGroup as DomainNewGroup};
use crate::domain::types::{GroupId, HubId, RecipientId};
use crate::models::group::{Group as DbGroup, GroupRecipient, NewGroup as DbNewGroup};
use crate::models::recipient::Recipient as DbRecipient;
use crate::repository::{DieselRepository, GroupListQuery, GroupReader, GroupWriter};

impl GroupReader for DieselRepository {
    fn get_group_by_id(
        &self,
        id: GroupId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<GroupWithRecipients>> {
        use crate::schema::{groups, groups_recipients, recipients};
        let mut conn = self.conn()?;

        conn.transaction::<Option<GroupWithRecipients>, RepositoryError, _>(|conn| {
            // Load group by id
            let db_group: Option<DbGroup> = groups::table
                .filter(groups::id.eq(id.get()))
                .filter(groups::hub_id.eq(hub_id.get()))
                .select(DbGroup::as_select())
                .first(conn)
                .optional()?;

            let db_group = match db_group {
                Some(g) => g,
                None => return Ok(None),
            };

            // Load recipient rows via join
            let db_recipients: Vec<DbRecipient> = groups_recipients::table
                .filter(groups_recipients::group_id.eq(id.get()))
                .inner_join(recipients::table)
                .select(DbRecipient::as_select())
                .load(conn)?;

            let recipients = hydrate_recipients(conn, hub_id, db_recipients)?;

            Ok(Some(GroupWithRecipients {
                group: DomainGroup::try_from(db_group)?,
                recipients,
            }))
        })
    }

    fn list_groups(&self, query: GroupListQuery) -> RepositoryResult<(usize, Vec<DomainGroup>)> {
        use crate::schema::groups;
        let mut conn = self.conn()?;

        conn.transaction::<(usize, Vec<DomainGroup>), RepositoryError, _>(|conn| {
            let query_builder = || {
                groups::table
                    .filter(groups::hub_id.eq(query.hub_id.get()))
                    .select(DbGroup::as_select())
                    .into_boxed::<diesel::sqlite::Sqlite>()
            };

            let total = query_builder().count().get_result::<i64>(conn)? as usize;

            let mut items = query_builder();
            items = apply_pagination(items, query.pagination.as_ref());

            // Final load
            let items = items
                .order(groups::name.asc())
                .load::<DbGroup>(conn)?
                .into_iter()
                .map(DomainGroup::try_from)
                .collect::<Result<Vec<DomainGroup>, _>>()?;

            Ok((total, items))
        })
    }
}

impl GroupWriter for DieselRepository {
    fn create_group(&self, group: &DomainNewGroup) -> RepositoryResult<DomainGroup> {
        use crate::schema::groups;
        let mut conn = self.conn()?;
        let db_new: DbNewGroup = group.into();
        let inserted = diesel::insert_into(groups::table)
            .values(&db_new)
            .get_result::<DbGroup>(&mut conn)?;
        Ok(DomainGroup::try_from(inserted)?)
    }

    fn delete_group(&self, id: GroupId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{groups, groups_recipients};
        let mut conn = self.conn()?;
        conn.transaction::<(), RepositoryError, _>(|conn| {
            let group_exists = groups::table
                .filter(groups::id.eq(id.get()))
                .filter(groups::hub_id.eq(hub_id.get()))
                .select(groups::id)
                .first::<i32>(conn)
                .optional()?;
            if group_exists.is_none() {
                return Err(RepositoryError::NotFound);
            }

            diesel::delete(
                groups_recipients::table.filter(groups_recipients::group_id.eq(id.get())),
            )
            .execute(conn)?;
            let deleted = diesel::delete(
                groups::table
                    .filter(groups::id.eq(id.get()))
                    .filter(groups::hub_id.eq(hub_id.get())),
            )
            .execute(conn)?;
            if deleted == 0 {
                return Err(RepositoryError::NotFound);
            }
            Ok(())
        })
    }

    fn delete_all_groups(&self, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{groups, groups_recipients};
        let mut conn = self.conn()?;

        conn.transaction::<(), RepositoryError, _>(|conn| {
            // Step 1: Get IDs of groups belonging to this hub
            let group_ids = groups::table
                .filter(groups::hub_id.eq(hub_id.get()))
                .select(groups::id)
                .load::<i32>(conn)?;

            // Step 2: Delete group_recipients entries for those group_ids
            diesel::delete(
                groups_recipients::table.filter(groups_recipients::group_id.eq_any(&group_ids)),
            )
            .execute(conn)?;

            // Step 3: Delete groups themselves
            diesel::delete(groups::table.filter(groups::hub_id.eq(hub_id.get()))).execute(conn)?;

            Ok(())
        })
    }

    fn assign_recipients_to_group(
        &self,
        group_id: GroupId,
        recipients: Vec<RecipientId>,
        hub_id: HubId,
    ) -> RepositoryResult<()> {
        use crate::schema::{groups, groups_recipients, recipients as recipients_table};
        let mut conn = self.conn()?;
        let unique_recipient_ids: Vec<i32> = recipients
            .iter()
            .map(|recipient_id| recipient_id.get())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let new: Vec<GroupRecipient> = unique_recipient_ids
            .iter()
            .map(|recipient_id| GroupRecipient {
                group_id: group_id.get(),
                recipient_id: *recipient_id,
            })
            .collect();

        conn.transaction::<(), RepositoryError, _>(|connection| {
            let group_exists = groups::table
                .filter(groups::id.eq(group_id.get()))
                .filter(groups::hub_id.eq(hub_id.get()))
                .select(groups::id)
                .first::<i32>(connection)
                .optional()?;
            if group_exists.is_none() {
                return Err(RepositoryError::NotFound);
            }

            if !unique_recipient_ids.is_empty() {
                let recipient_count = recipients_table::table
                    .filter(recipients_table::id.eq_any(&unique_recipient_ids))
                    .filter(recipients_table::hub_id.eq(hub_id.get()))
                    .count()
                    .get_result::<i64>(connection)? as usize;
                if recipient_count != unique_recipient_ids.len() {
                    return Err(RepositoryError::NotFound);
                }
            }

            // Step 1: Delete group_recipients entries for the group_id
            diesel::delete(
                groups_recipients::table.filter(groups_recipients::group_id.eq(group_id.get())),
            )
            .execute(connection)?;

            // Step 2: Assign new group_recipients entries for the group_id
            if !new.is_empty() {
                diesel::insert_into(groups_recipients::table)
                    .values(&new)
                    .execute(connection)?;
            }

            Ok(())
        })
    }
}
