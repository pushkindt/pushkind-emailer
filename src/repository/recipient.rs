use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use diesel::upsert::excluded;
use pushkind_common::repository::build_fts_match_query;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use super::helpers::{apply_pagination, hydrate_recipients};
use crate::domain::group::Group as DomainGroup;
use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Recipient as DomainRecipient, RecipientWithGroups,
    Unsubscribe as DomainUnsubscribe, UpdateRecipient as DomainUpdateRecipient,
};
use crate::models::group::{Group, GroupRecipient};
use crate::models::recipient::{NewRecipient, Recipient, RecipientField, Unsubscribe};
use crate::repository::{DieselRepository, RecipientListQuery, RecipientReader, RecipientWriter};

impl RecipientReader for DieselRepository {
    fn get_recipient_by_id(
        &self,
        id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Option<RecipientWithGroups>> {
        use crate::schema::{groups, recipients, unsubscribes};

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

        let unsubscribed_at = unsubscribes::table
            .filter(unsubscribes::email.eq(&recipient.email))
            .filter(unsubscribes::hub_id.eq(recipient.hub_id))
            .select(unsubscribes::created_at)
            .first::<chrono::NaiveDateTime>(&mut conn)
            .optional()?;

        let group_ids = groups.iter().map(|gr| gr.id).collect::<Vec<_>>();
        let domain_recipient = DomainRecipient::try_new(
            recipient.id,
            recipient.name,
            recipient.email,
            recipient.hub_id,
            field_map,
            recipient.created_at,
            recipient.updated_at,
            unsubscribed_at,
            group_ids,
        )
        .map_err(|err| RepositoryError::ValidationError(err.to_string()))?;

        Ok(Some(RecipientWithGroups {
            recipient: domain_recipient,
            groups: groups
                .into_iter()
                .map(DomainGroup::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| RepositoryError::ValidationError(err.to_string()))?,
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
        items = apply_pagination(items, query.pagination.as_ref());

        // Load recipients for the hub
        let db_recipients: Vec<Recipient> = items.order(recipients::name.desc()).load(&mut conn)?;
        let recipients = hydrate_recipients(&mut conn, query.hub_id, db_recipients)?;

        Ok((total, recipients))
    }

    fn search_recipients(
        &self,
        query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainRecipient>)> {
        use crate::models::recipient::RecipientCount;

        let mut conn = self.conn()?;

        // Prepare a safe FTS5 MATCH query using helper
        let match_query = match &query.search {
            None => return Ok((0, vec![])),
            Some(raw) => match build_fts_match_query(raw) {
                Some(mq) => mq,
                None => return Ok((0, vec![])),
            },
        };

        // Build base SQL
        let mut sql = String::from(
            r#"
            SELECT recipients.*
            FROM recipients
            JOIN recipient_fts ON recipients.id = recipient_fts.rowid
            WHERE recipient_fts MATCH ?
            AND recipients.hub_id = ?
            "#,
        );

        let total_sql = format!("SELECT COUNT(*) as count FROM ({sql})");

        // Now add pagination to SQL (but not count)
        if query.pagination.is_some() {
            sql.push_str(" LIMIT ? OFFSET ? ");
        }

        // Build final data query
        let mut data_query = diesel::sql_query(&sql)
            .into_boxed()
            .bind::<Text, _>(&match_query)
            .bind::<Integer, _>(query.hub_id);

        let total_query = diesel::sql_query(&total_sql)
            .into_boxed()
            .bind::<Text, _>(&match_query)
            .bind::<Integer, _>(query.hub_id);

        if let Some(pagination) = &query.pagination {
            let limit = pagination.per_page as i64;
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            data_query = data_query
                .bind::<BigInt, _>(limit)
                .bind::<BigInt, _>(offset);
        }

        let db_recipients = data_query.load::<Recipient>(&mut conn)?;

        let total = total_query.get_result::<RecipientCount>(&mut conn)?.count as usize;
        let recipients = hydrate_recipients(&mut conn, query.hub_id, db_recipients)?;

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

    fn list_unsubscribed_recipients(
        &self,
        hub_id: i32,
    ) -> RepositoryResult<Vec<DomainUnsubscribe>> {
        use crate::schema::unsubscribes;

        let mut conn = self.conn()?;

        let results = unsubscribes::table
            .filter(unsubscribes::hub_id.eq(hub_id))
            .select(Unsubscribe::as_select())
            .order(unsubscribes::created_at.desc())
            .load::<Unsubscribe>(&mut conn)?;

        results
            .into_iter()
            .map(DomainUnsubscribe::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| RepositoryError::ValidationError(err.to_string()))
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
                    .set((recipients::name.eq(new.name.as_str()),))
                    .get_result::<Recipient>(conn)?;

                // Update fields (delete all → insert new)
                diesel::delete(
                    recipient_fields::table.filter(recipient_fields::recipient_id.eq(inserted.id)),
                )
                .execute(conn)?;

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

                // Update denormalized `recipients.fields` using a Diesel subselect
                diesel::update(recipients::table.find(inserted.id))
                    .set(
                        recipients::fields.eq(recipient_fields::table
                            .filter(recipient_fields::recipient_id.eq(inserted.id))
                            .select(diesel::dsl::sql::<Nullable<Text>>(
                                "trim(COALESCE(group_concat(value, ' '), ''))",
                            ))
                            .single_value()),
                    )
                    .execute(conn)?;

                // Update group associations (delete all → insert new)
                diesel::delete(
                    groups_recipients::table
                        .filter(groups_recipients::recipient_id.eq(inserted.id)),
                )
                .execute(conn)?;

                // Create and assign groups
                if let Some(names) = &new.groups {
                    for group_name in names {
                        // Check if group already exists
                        let existing = groups::table
                            .filter(groups::name.eq(group_name))
                            .filter(groups::hub_id.eq(new.hub_id.get()))
                            .select(Group::as_select())
                            .first::<Group>(conn)
                            .optional()?;

                        let group = match existing {
                            Some(g) => g,
                            None => {
                                let new_group = crate::models::group::NewGroup {
                                    name: group_name,
                                    hub_id: new.hub_id.get(),
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
        use crate::schema::{groups_recipients, recipient_fields, recipients, unsubscribes};
        let mut conn = self.conn()?;

        // Update basic recipient info
        diesel::update(recipients::table.filter(recipients::id.eq(id)))
            .set((
                recipients::name.eq(recipient.name.as_str()),
                recipients::email.eq(recipient.email.as_str()),
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

        // Update denormalized `recipients.fields` using a Diesel subselect
        diesel::update(recipients::table.find(id))
            .set(
                recipients::fields.eq(recipient_fields::table
                    .filter(recipient_fields::recipient_id.eq(id))
                    .select(diesel::dsl::sql::<Nullable<Text>>(
                        "trim(COALESCE(group_concat(value, ' '), ''))",
                    ))
                    .single_value()),
            )
            .execute(&mut conn)?;

        // Update group associations (delete all → insert new)
        diesel::delete(groups_recipients::table.filter(groups_recipients::recipient_id.eq(id)))
            .execute(&mut conn)?;
        for group_id in &recipient.groups {
            let link = GroupRecipient {
                group_id: group_id.get(),
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

        let unsubscribed_at = unsubscribes::table
            .filter(unsubscribes::hub_id.eq(rec.hub_id))
            .filter(unsubscribes::email.eq(&rec.email))
            .select(unsubscribes::created_at)
            .first::<chrono::NaiveDateTime>(&mut conn)
            .optional()?;

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

        DomainRecipient::try_new(
            rec.id,
            rec.name,
            rec.email,
            rec.hub_id,
            fields_map,
            rec.created_at,
            rec.updated_at,
            unsubscribed_at,
            group_ids,
        )
        .map_err(|err| RepositoryError::ValidationError(err.to_string()))
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
