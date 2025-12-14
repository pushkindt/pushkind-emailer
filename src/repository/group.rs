use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use super::helpers::{apply_pagination, hydrate_recipients};
use crate::domain::group::{Group as DomainGroup, GroupWithRecipients, NewGroup as DomainNewGroup};
use crate::models::group::{Group as DbGroup, GroupRecipient, NewGroup as DbNewGroup};
use crate::models::recipient::Recipient as DbRecipient;
use crate::repository::{DieselRepository, GroupListQuery, GroupReader, GroupWriter};

impl GroupReader for DieselRepository {
    fn get_group_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<GroupWithRecipients>> {
        use crate::schema::{groups, groups_recipients, recipients};
        let mut conn = self.conn()?;

        // Load group by id
        let db_group: Option<DbGroup> = groups::table
            .filter(groups::id.eq(id))
            .filter(groups::hub_id.eq(hub_id))
            .select(DbGroup::as_select())
            .first(&mut conn)
            .optional()?;

        let db_group = match db_group {
            Some(g) => g,
            None => return Ok(None),
        };

        // Load recipient rows via join
        let db_recipients: Vec<DbRecipient> = groups_recipients::table
            .filter(groups_recipients::group_id.eq(id))
            .inner_join(recipients::table)
            .select(DbRecipient::as_select())
            .load(&mut conn)?;

        let recipients = hydrate_recipients(&mut conn, hub_id, db_recipients)?;

        Ok(Some(GroupWithRecipients {
            group: DomainGroup::try_from(db_group)
                .map_err(|err| RepositoryError::ValidationError(err.to_string()))?,
            recipients,
        }))
    }

    fn list_groups(&self, query: GroupListQuery) -> RepositoryResult<(usize, Vec<DomainGroup>)> {
        use crate::schema::groups;
        let mut conn = self.conn()?;

        let query_builder = || {
            groups::table
                .filter(groups::hub_id.eq(query.hub_id))
                .select(DbGroup::as_select())
                .into_boxed::<diesel::sqlite::Sqlite>()
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();
        items = apply_pagination(items, query.pagination.as_ref());

        // Final load
        let items = items
            .order(groups::name.asc())
            .load::<DbGroup>(&mut conn)?
            .into_iter()
            .map(DomainGroup::try_from)
            .collect::<Result<Vec<DomainGroup>, _>>()
            .map_err(|err| RepositoryError::ValidationError(err.to_string()))?;

        Ok((total, items))
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
        DomainGroup::try_from(inserted)
            .map_err(|err| RepositoryError::ValidationError(err.to_string()))
    }

    fn delete_group(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::{groups, groups_recipients};
        let mut conn = self.conn()?;
        diesel::delete(groups_recipients::table.filter(groups_recipients::group_id.eq(id)))
            .execute(&mut conn)?;
        diesel::delete(groups::table.filter(groups::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }

    fn delete_all_groups(&self, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::{groups, groups_recipients};
        let mut conn = self.conn()?;

        // Step 1: Get IDs of groups belonging to this hub
        let group_ids = groups::table
            .filter(groups::hub_id.eq(hub_id))
            .select(groups::id)
            .load::<i32>(&mut conn)?;

        // Step 2: Delete group_recipients entries for those group_ids
        diesel::delete(
            groups_recipients::table.filter(groups_recipients::group_id.eq_any(&group_ids)),
        )
        .execute(&mut conn)?;

        // Step 3: Delete groups themselves
        diesel::delete(groups::table.filter(groups::hub_id.eq(hub_id))).execute(&mut conn)?;

        Ok(())
    }

    fn assign_recipients_to_group(
        &self,
        group_id: i32,
        recipients: Vec<i32>,
    ) -> RepositoryResult<()> {
        use crate::schema::groups_recipients;
        let mut conn = self.conn()?;
        let new: Vec<GroupRecipient> = recipients
            .iter()
            .map(|recipient_id| GroupRecipient {
                group_id,
                recipient_id: *recipient_id,
            })
            .collect();

        conn.transaction(|connection| {
            // Step 1: Delete group_recipients entries for the group_id
            diesel::delete(
                groups_recipients::table.filter(groups_recipients::group_id.eq(group_id)),
            )
            .execute(connection)?;

            // Step 2: Assign new group_recipients entries for the group_id
            diesel::insert_into(groups_recipients::table)
                .values(&new)
                .execute(connection)?;

            diesel::result::QueryResult::Ok(())
        })?;
        Ok(())
    }
}
