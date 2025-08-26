use std::collections::HashMap;

use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::group::{Group as DomainGroup, GroupWithRecipients, NewGroup as DomainNewGroup};
use crate::domain::recipient::Recipient as DomainRecipient;
use crate::models::group::{Group as DbGroup, GroupRecipient, NewGroup as DbNewGroup};
use crate::models::recipient::{Recipient as DbRecipient, RecipientField};
use crate::repository::{DieselRepository, GroupReader, GroupWriter};

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

        if db_recipients.is_empty() {
            return Ok(Some(GroupWithRecipients {
                group: DomainGroup::from(db_group),
                recipients: vec![],
            }));
        }

        // Load recipient fields grouped by recipient
        let db_fields = RecipientField::belonging_to(&db_recipients)
            .select(RecipientField::as_select())
            .load::<RecipientField>(&mut conn)?
            .grouped_by(&db_recipients);

        // Load group memberships for each recipient
        let db_group_links = GroupRecipient::belonging_to(&db_recipients)
            .select(GroupRecipient::as_select())
            .load::<GroupRecipient>(&mut conn)?;

        let mut recipient_to_group_ids: HashMap<i32, Vec<i32>> = HashMap::new();
        for link in db_group_links {
            recipient_to_group_ids
                .entry(link.recipient_id)
                .or_default()
                .push(link.group_id);
        }

        // Compose domain recipients
        let recipients = db_recipients
            .into_iter()
            .zip(db_fields)
            .map(|(r, fields)| DomainRecipient {
                id: r.id,
                name: r.name,
                email: r.email,
                hub_id: r.hub_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
                unsubscribed_at: r.unsubscribed_at,
                fields: fields
                    .into_iter()
                    .map(|f| (f.field, f.value))
                    .collect::<HashMap<_, _>>(),
                groups: recipient_to_group_ids.remove(&r.id).unwrap_or_default(),
            })
            .collect();

        Ok(Some(GroupWithRecipients {
            group: DomainGroup::from(db_group),
            recipients,
        }))
    }

    fn list_groups(&self, hub_id: i32) -> RepositoryResult<Vec<DomainGroup>> {
        use crate::schema::groups;
        let mut conn = self.conn()?;

        // Fetch groups for hub
        let db_groups: Vec<DbGroup> = groups::table
            .filter(groups::hub_id.eq(hub_id))
            .select(DbGroup::as_select())
            .load(&mut conn)?;

        Ok(db_groups.into_iter().map(|g| g.into()).collect())
    }
}

impl GroupWriter for DieselRepository {
    fn create_group(&self, group: &DomainNewGroup) -> RepositoryResult<DomainGroup> {
        use crate::schema::groups;
        let mut conn = self.conn()?;
        let db_new = DbNewGroup {
            name: group.name,
            hub_id: group.hub_id,
        };
        let inserted = diesel::insert_into(groups::table)
            .values(&db_new)
            .get_result::<DbGroup>(&mut conn)?;
        Ok(inserted.into())
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

    fn assign_recipient_to_group(&self, group_id: i32, recipient_id: i32) -> RepositoryResult<()> {
        use crate::schema::groups_recipients;
        let mut conn = self.conn()?;
        let new = GroupRecipient {
            group_id,
            recipient_id,
        };
        diesel::insert_into(groups_recipients::table)
            .values(&new)
            .execute(&mut conn)?;
        Ok(())
    }

    fn unassign_recipient_to_group(
        &self,
        group_id: i32,
        recipient_id: i32,
    ) -> RepositoryResult<()> {
        use crate::schema::groups_recipients;

        let mut conn = self.conn()?;

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
