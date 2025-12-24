//! Repository operations for recipients and subscriptions.
use std::collections::BTreeMap;

use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Nullable, Text};
use diesel::upsert::excluded;
use pushkind_common::repository::build_fts_match_query;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use super::helpers::{apply_pagination, hydrate_recipients};
use crate::domain::group::Group as DomainGroup;
use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Recipient as DomainRecipient, RecipientWithGroups,
    Unsubscribe as DomainUnsubscribe, UpdateRecipient as DomainUpdateRecipient,
};
use crate::domain::types::{HubId, RecipientId};
use crate::models::group::{Group, GroupRecipient};
use crate::models::recipient::{NewRecipient, Recipient, RecipientField, Unsubscribe};
use crate::repository::{DieselRepository, RecipientListQuery, RecipientReader, RecipientWriter};

impl RecipientReader for DieselRepository {
    fn get_recipient_by_id(
        &self,
        id: RecipientId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<RecipientWithGroups>> {
        use crate::schema::{groups, recipients, unsubscribes};

        let mut conn = self.conn()?;

        conn.transaction::<Option<RecipientWithGroups>, RepositoryError, _>(|conn| {
            let recipient = recipients::table
                .filter(recipients::id.eq(id.get()))
                .filter(recipients::hub_id.eq(hub_id.get()))
                .first::<Recipient>(conn)
                .optional()?;
            let recipient = match recipient {
                Some(recipient) => recipient,
                None => return Ok(None),
            };

            let groups = GroupRecipient::belonging_to(&recipient)
                .inner_join(groups::table)
                .select(Group::as_select())
                .load::<Group>(conn)?;

            let fields = RecipientField::belonging_to(&recipient)
                .select(RecipientField::as_select())
                .load::<RecipientField>(conn)?;

            let field_map = fields.into_iter().map(|f| (f.field, f.value)).collect();

            let unsubscribed_at = unsubscribes::table
                .filter(unsubscribes::email.eq(&recipient.email))
                .filter(unsubscribes::hub_id.eq(recipient.hub_id))
                .select(unsubscribes::created_at)
                .first::<chrono::NaiveDateTime>(conn)
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
            )?;

            Ok(Some(RecipientWithGroups {
                recipient: domain_recipient,
                groups: groups
                    .into_iter()
                    .map(DomainGroup::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        })
    }

    fn list_recipients(
        &self,
        query: RecipientListQuery,
    ) -> RepositoryResult<(usize, Vec<DomainRecipient>)> {
        use crate::schema::{groups_recipients, recipient_fts, recipients};
        let mut conn = self.conn()?;

        conn.transaction::<(usize, Vec<DomainRecipient>), RepositoryError, _>(|conn| {
            let query_builder = || {
                let mut items = recipients::table
                    .filter(recipients::hub_id.eq(query.hub_id.get()))
                    .select(Recipient::as_select())
                    .into_boxed::<diesel::sqlite::Sqlite>();

                if let Some(emails) = query.emails.as_ref() {
                    items =
                        items.filter(recipients::email.eq_any(emails.iter().map(|e| e.as_str())));
                }
                if let Some(group_ids) = query.group_ids.as_ref() {
                    items = items.filter(
                        recipients::id.eq_any(
                            groups_recipients::table
                                .filter(
                                    groups_recipients::group_id
                                        .eq_any(group_ids.iter().map(|g| g.get())),
                                )
                                .select(groups_recipients::recipient_id),
                        ),
                    );
                }

                if let Some(term) = query.search.as_ref()
                    && let Some(fts_query) = build_fts_match_query(term)
                {
                    let fts_filter = exists(
                        recipient_fts::table
                            .filter(recipient_fts::rowid.eq(recipients::id))
                            .filter(
                                diesel::dsl::sql::<Bool>("recipient_fts MATCH ")
                                    .bind::<Text, _>(fts_query),
                            ),
                    );
                    items = items.filter(fts_filter);
                }

                items
            };

            let total = query_builder().count().get_result::<i64>(conn)? as usize;

            let mut items = query_builder();
            items = apply_pagination(items, query.pagination.as_ref());

            // Load recipients for the hub
            let db_recipients: Vec<Recipient> = items.order(recipients::name.desc()).load(conn)?;
            let recipients = hydrate_recipients(conn, query.hub_id, db_recipients)?;

            Ok((total, recipients))
        })
    }

    fn list_custom_fields(&self, hub_id: HubId) -> RepositoryResult<Vec<String>> {
        use crate::schema::{recipient_fields, recipients};

        let mut conn = self.conn()?;

        let fields: Vec<String> = recipient_fields::table
            .inner_join(recipients::table)
            .filter(recipients::hub_id.eq(hub_id.get()))
            .select(recipient_fields::field)
            .distinct()
            .order(recipient_fields::field.asc())
            .load(&mut conn)?;

        Ok(fields)
    }

    fn list_unsubscribed_recipients(
        &self,
        hub_id: HubId,
    ) -> RepositoryResult<Vec<DomainUnsubscribe>> {
        use crate::schema::unsubscribes;

        let mut conn = self.conn()?;

        let results = unsubscribes::table
            .filter(unsubscribes::hub_id.eq(hub_id.get()))
            .select(Unsubscribe::as_select())
            .order(unsubscribes::created_at.desc())
            .load::<Unsubscribe>(&mut conn)?;

        let recipients = results
            .into_iter()
            .map(DomainUnsubscribe::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(recipients)
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
                            .filter(groups::name.eq(group_name.as_str()))
                            .filter(groups::hub_id.eq(new.hub_id.get()))
                            .select(Group::as_select())
                            .first::<Group>(conn)
                            .optional()?;

                        let group = match existing {
                            Some(g) => g,
                            None => {
                                let new_group = crate::models::group::NewGroup {
                                    name: group_name.as_str(),
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
        id: RecipientId,
        hub_id: HubId,
        recipient: &DomainUpdateRecipient,
    ) -> RepositoryResult<DomainRecipient> {
        use crate::schema::{groups_recipients, recipient_fields, recipients, unsubscribes};
        let mut conn = self.conn()?;

        conn.transaction::<DomainRecipient, RepositoryError, _>(|conn| {
            // Update basic recipient info
            let updated = diesel::update(
                recipients::table
                    .filter(recipients::id.eq(id.get()))
                    .filter(recipients::hub_id.eq(hub_id.get())),
            )
            .set((
                recipients::name.eq(recipient.name.as_str()),
                recipients::email.eq(recipient.email.as_str()),
                ))
                .execute(conn)?;
            if updated == 0 {
                return Err(RepositoryError::NotFound);
            }

            // Update fields (delete all → insert new)
            diesel::delete(
                recipient_fields::table.filter(recipient_fields::recipient_id.eq(id.get())),
            )
            .execute(conn)?;
            for (field, value) in &recipient.fields {
                let new_field = RecipientField {
                    recipient_id: id.get(),
                    field: field.clone(),
                    value: value.clone(),
                };
                diesel::insert_into(recipient_fields::table)
                    .values(&new_field)
                    .execute(conn)?;
            }

            // Update denormalized `recipients.fields` using a Diesel subselect
            diesel::update(recipients::table.find(id.get()))
                .set(
                    recipients::fields.eq(recipient_fields::table
                        .filter(recipient_fields::recipient_id.eq(id.get()))
                        .select(diesel::dsl::sql::<Nullable<Text>>(
                            "trim(COALESCE(group_concat(value, ' '), ''))",
                        ))
                        .single_value()),
                )
                .execute(conn)?;

            // Update group associations (delete all → insert new)
            diesel::delete(
                groups_recipients::table.filter(groups_recipients::recipient_id.eq(id.get())),
            )
            .execute(conn)?;
            for group_id in &recipient.groups {
                let link = GroupRecipient {
                    group_id: group_id.get(),
                    recipient_id: id.get(),
                };
                diesel::insert_into(groups_recipients::table)
                    .values(&link)
                    .execute(conn)?;
            }

            // Reload the updated recipient
            let rec = recipients::table
                .filter(recipients::id.eq(id.get()))
                .filter(recipients::hub_id.eq(hub_id.get()))
                .select(Recipient::as_select())
                .first::<Recipient>(conn)?;

            let unsubscribed_at = unsubscribes::table
                .filter(unsubscribes::hub_id.eq(rec.hub_id))
                .filter(unsubscribes::email.eq(&rec.email))
                .select(unsubscribes::created_at)
                .first::<chrono::NaiveDateTime>(conn)
                .optional()?;

            // Reload fields
            let fields_vec = recipient_fields::table
                .filter(recipient_fields::recipient_id.eq(id.get()))
                .select(RecipientField::as_select())
                .load::<RecipientField>(conn)?;

            let fields_map = fields_vec
                .into_iter()
                .map(|f| (f.field, f.value))
                .collect::<BTreeMap<_, _>>();

            // Reload group IDs
            let group_ids = groups_recipients::table
                .filter(groups_recipients::recipient_id.eq(id.get()))
                .select(groups_recipients::group_id)
                .load::<i32>(conn)?;

            Ok(DomainRecipient::try_new(
                rec.id,
                rec.name,
                rec.email,
                rec.hub_id,
                fields_map,
                rec.created_at,
                rec.updated_at,
                unsubscribed_at,
                group_ids,
            )?)
        })
    }

    fn delete_recipient(&self, id: RecipientId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.conn()?;
        conn.transaction::<(), RepositoryError, _>(|conn| {
            let recipient_exists = recipients::table
                .filter(recipients::id.eq(id.get()))
                .filter(recipients::hub_id.eq(hub_id.get()))
                .select(recipients::id)
                .first::<i32>(conn)
                .optional()?;
            if recipient_exists.is_none() {
                return Err(RepositoryError::NotFound);
            }

            diesel::delete(
                groups_recipients::table.filter(groups_recipients::recipient_id.eq(id.get())),
            )
            .execute(conn)?;
            diesel::delete(
                recipient_fields::table.filter(recipient_fields::recipient_id.eq(id.get())),
            )
            .execute(conn)?;
            let deleted = diesel::delete(
                recipients::table
                    .filter(recipients::id.eq(id.get()))
                    .filter(recipients::hub_id.eq(hub_id.get())),
            )
            .execute(conn)?;
            if deleted == 0 {
                return Err(RepositoryError::NotFound);
            }
            Ok(())
        })
    }

    fn delete_all_recipients(&self, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.conn()?;

        conn.transaction::<(), RepositoryError, _>(|conn| {
            // Step 1: Find recipient IDs for the given hub
            let recipient_ids = recipients::table
                .filter(recipients::hub_id.eq(hub_id.get()))
                .select(recipients::id)
                .load::<i32>(conn)?;

            // Step 2: Delete group_recipients entries for these recipients
            diesel::delete(
                groups_recipients::table
                    .filter(groups_recipients::recipient_id.eq_any(&recipient_ids)),
            )
            .execute(conn)?;

            // Step 3: Delete recipient_fields entries for these recipients
            diesel::delete(
                recipient_fields::table
                    .filter(recipient_fields::recipient_id.eq_any(&recipient_ids)),
            )
            .execute(conn)?;

            // Step 4: Delete the recipients themselves
            diesel::delete(recipients::table.filter(recipients::hub_id.eq(hub_id.get())))
                .execute(conn)?;

            Ok(())
        })
    }
}
