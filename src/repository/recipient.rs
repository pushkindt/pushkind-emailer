use std::collections::HashMap;

use diesel::prelude::*;
use diesel::upsert::excluded;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Recipient as DomainRecipient, RecipientWithGroups,
    UpdateRecipient as DomainUpdateRecipient,
};
use crate::models::group::{Group, GroupRecipient};
use crate::models::recipient::{NewRecipient, Recipient, RecipientField};
use crate::repository::{DieselRepository, RecipientListQuery, RecipientReader, RecipientWriter};

impl RecipientReader for DieselRepository {
    fn get_recipient_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<RecipientWithGroups>> {
        use crate::schema::{groups, recipients};

        let mut conn = self.conn()?;

        let recipient = recipients::table
            .filter(recipients::id.eq(id))
            .filter(recipients::hub_id.eq(hub_id))
            .first::<Recipient>(&mut conn)
            .optional()?;
        let recipient = match recipient {
            Some(recipient) => recipient,
            None => return Ok(None),
        };

        let groups = GroupRecipient::belonging_to(&recipient)
            .inner_join(groups::table)
            .select(Group::as_select())
            .load::<Group>(&mut conn)?;

        let fields = RecipientField::belonging_to(&recipient)
            .select(RecipientField::as_select())
            .load::<RecipientField>(&mut conn)?;

        let field_map = fields.into_iter().map(|f| (f.field, f.value)).collect();

        Ok(Some(RecipientWithGroups {
            recipient: DomainRecipient {
                id: recipient.id,
                name: recipient.name,
                email: recipient.email,
                hub_id: recipient.hub_id,
                fields: field_map,
                created_at: recipient.created_at,
                updated_at: recipient.updated_at,
                unsubscribed_at: recipient.unsubscribed_at,
                groups: groups.iter().map(|gr| gr.id).collect(),
            },
            groups: groups.into_iter().map(|gr| gr.into()).collect(),
        }))
    }

    fn list_recipients(
        &self,
        query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainRecipient>)> {
        use crate::schema::{groups_recipients, recipients};
        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = recipients::table
                .filter(recipients::hub_id.eq(query.hub_id))
                .select(Recipient::as_select())
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(emails) = query.emails.as_ref() {
                items = items.filter(recipients::email.eq_any(emails));
            }
            if let Some(group_ids) = query.group_ids.as_ref() {
                items = items.filter(
                    recipients::id.eq_any(
                        groups_recipients::table
                            .filter(groups_recipients::group_id.eq_any(group_ids))
                            .select(groups_recipients::recipient_id),
                    ),
                );
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();

        // Apply pagination if requested
        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        // Load recipients for the hub
        let db_recipients: Vec<Recipient> = items.order(recipients::name.desc()).load(&mut conn)?;

        if db_recipients.is_empty() {
            return Ok((total, Vec::new()));
        }

        // Load recipient fields, grouped by recipient
        let db_fields = RecipientField::belonging_to(&db_recipients)
            .select(RecipientField::as_select())
            .load::<RecipientField>(&mut conn)?
            .grouped_by(&db_recipients);

        // Load group-recipient relations
        let db_group_recipients = GroupRecipient::belonging_to(&db_recipients)
            .select(GroupRecipient::as_select())
            .load::<GroupRecipient>(&mut conn)?;

        // Build a map from recipient_id to group IDs
        let mut recipient_id_to_group_ids: HashMap<i32, Vec<i32>> = HashMap::new();
        for relation in db_group_recipients {
            recipient_id_to_group_ids
                .entry(relation.recipient_id)
                .or_default()
                .push(relation.group_id);
        }

        // Compose DomainRecipient
        let recipients: Vec<DomainRecipient> = db_recipients
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
                groups: recipient_id_to_group_ids.remove(&r.id).unwrap_or_default(),
            })
            .collect();

        Ok((total, recipients))
    }

    fn list_custom_fields(&self, hub_id: i32) -> RepositoryResult<Vec<String>> {
        use crate::schema::{recipient_fields, recipients};

        let mut conn = self.conn()?;

        let fields: Vec<String> = recipient_fields::table
            .inner_join(recipients::table)
            .filter(recipients::hub_id.eq(hub_id))
            .select(recipient_fields::field)
            .distinct()
            .order(recipient_fields::field.asc())
            .load(&mut conn)?;

        Ok(fields)
    }
}

impl RecipientWriter for DieselRepository {
    fn create_recipients(&self, recipient: &[DomainNewRecipient]) -> RepositoryResult<usize> {
        use crate::schema::{groups, groups_recipients, recipient_fields, recipients};

        let mut conn = self.conn()?;

        conn.transaction::<usize, RepositoryError, _>(|conn| {
            let mut count_inserted: usize = 0;

            for new in recipient {
                let db_new: NewRecipient = new.into();

                let inserted = diesel::insert_into(recipients::table)
                    .values(&db_new)
                    .on_conflict((recipients::email, recipients::hub_id))
                    .do_update()
                    .set((recipients::name.eq(&new.name),))
                    .get_result::<Recipient>(conn)?;

                // Insert optional fields
                if let Some(fields) = &new.fields {
                    let new_fields: Vec<RecipientField> = fields
                        .iter()
                        .map(|(f, v)| RecipientField {
                            recipient_id: inserted.id,
                            field: f.clone(),
                            value: v.clone(),
                        })
                        .collect();
                    if !new_fields.is_empty() {
                        for field in new_fields {
                            diesel::insert_into(recipient_fields::table)
                                .values(&field)
                                .on_conflict((
                                    recipient_fields::recipient_id,
                                    recipient_fields::field,
                                ))
                                .do_update()
                                .set(recipient_fields::value.eq(excluded(recipient_fields::value)))
                                .execute(conn)?;
                        }
                    }
                }

                // Create and assign groups
                if let Some(names) = &new.groups {
                    for group_name in names {
                        // Check if group already exists
                        let existing = groups::table
                            .filter(groups::name.eq(group_name))
                            .filter(groups::hub_id.eq(new.hub_id))
                            .select(Group::as_select())
                            .first::<Group>(conn)
                            .optional()?;

                        let group = match existing {
                            Some(g) => g,
                            None => {
                                let new_group = crate::models::group::NewGroup {
                                    name: group_name,
                                    hub_id: new.hub_id,
                                };
                                diesel::insert_into(groups::table)
                                    .values(&new_group)
                                    .get_result::<Group>(conn)?
                            }
                        };

                        let link = GroupRecipient {
                            group_id: group.id,
                            recipient_id: inserted.id,
                        };
                        diesel::insert_into(groups_recipients::table)
                            .values(&link)
                            .execute(conn)?;
                    }
                }

                count_inserted += 1;
            }

            Ok(count_inserted)
        })
    }

    fn update_recipient(
        &self,
        id: i32,
        recipient: &DomainUpdateRecipient,
    ) -> RepositoryResult<DomainRecipient> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.conn()?;

        // Update basic recipient info
        diesel::update(recipients::table.filter(recipients::id.eq(id)))
            .set((
                recipients::name.eq(&recipient.name),
                recipients::email.eq(&recipient.email),
                recipients::unsubscribed_at.eq(recipient.unsubscribed_at),
            ))
            .execute(&mut conn)?;

        // Update fields (delete all → insert new)
        diesel::delete(recipient_fields::table.filter(recipient_fields::recipient_id.eq(id)))
            .execute(&mut conn)?;
        for (field, value) in &recipient.fields {
            let new_field = RecipientField {
                recipient_id: id,
                field: field.clone(),
                value: value.clone(),
            };
            diesel::insert_into(recipient_fields::table)
                .values(&new_field)
                .execute(&mut conn)?;
        }

        // Update group associations (delete all → insert new)
        diesel::delete(groups_recipients::table.filter(groups_recipients::recipient_id.eq(id)))
            .execute(&mut conn)?;
        for group_id in &recipient.groups {
            let link = GroupRecipient {
                group_id: *group_id,
                recipient_id: id,
            };
            diesel::insert_into(groups_recipients::table)
                .values(&link)
                .execute(&mut conn)?;
        }

        // Reload the updated recipient
        let rec = recipients::table
            .filter(recipients::id.eq(id))
            .select(Recipient::as_select())
            .first::<Recipient>(&mut conn)?;

        // Reload fields
        let fields_vec = recipient_fields::table
            .filter(recipient_fields::recipient_id.eq(id))
            .select(RecipientField::as_select())
            .load::<RecipientField>(&mut conn)?;

        let fields_map = fields_vec
            .into_iter()
            .map(|f| (f.field, f.value))
            .collect::<HashMap<_, _>>();

        // Reload group IDs
        let group_ids = groups_recipients::table
            .filter(groups_recipients::recipient_id.eq(id))
            .select(groups_recipients::group_id)
            .load::<i32>(&mut conn)?;

        Ok(DomainRecipient {
            id: rec.id,
            name: rec.name,
            email: rec.email,
            hub_id: rec.hub_id,
            fields: fields_map,
            created_at: rec.created_at,
            updated_at: rec.updated_at,
            unsubscribed_at: rec.unsubscribed_at,
            groups: group_ids,
        })
    }

    fn delete_recipient(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.conn()?;
        diesel::delete(groups_recipients::table.filter(groups_recipients::recipient_id.eq(id)))
            .execute(&mut conn)?;
        diesel::delete(recipient_fields::table.filter(recipient_fields::recipient_id.eq(id)))
            .execute(&mut conn)?;
        diesel::delete(recipients::table.filter(recipients::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }

    fn delete_all_recipients(&self, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.conn()?;

        // Step 1: Find recipient IDs for the given hub
        let recipient_ids = recipients::table
            .filter(recipients::hub_id.eq(hub_id))
            .select(recipients::id)
            .load::<i32>(&mut conn)?;

        // Step 2: Delete group_recipients entries for these recipients
        diesel::delete(
            groups_recipients::table.filter(groups_recipients::recipient_id.eq_any(&recipient_ids)),
        )
        .execute(&mut conn)?;

        // Step 3: Delete recipient_fields entries for these recipients
        diesel::delete(
            recipient_fields::table.filter(recipient_fields::recipient_id.eq_any(&recipient_ids)),
        )
        .execute(&mut conn)?;

        // Step 4: Delete the recipients themselves
        diesel::delete(recipients::table.filter(recipients::hub_id.eq(hub_id)))
            .execute(&mut conn)?;

        Ok(())
    }
}
