use pushkind_common::db::DbPool;
use std::collections::HashMap;

use diesel::prelude::*;

use crate::domain::recipient::{
    NewRecipient as DomainNewRecipient, Recipient as DomainRecipient, RecipientWithGroups,
    UpdateRecipient as DomainUpdateRecipient,
};
use crate::models::group::{Group, GroupRecipient};
use crate::models::recipient::{NewRecipient, Recipient, RecipientField};
use crate::repository::errors::{RepositoryError, RepositoryResult};
use crate::repository::{RecipientReader, RecipientWriter};

/// Diesel implementation of [`RecipientRepository`].
pub struct DieselRecipientRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselRecipientRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl RecipientReader for DieselRecipientRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<RecipientWithGroups>> {
        use crate::schema::{groups, recipients};

        let mut conn = self.pool.get()?;

        let recipient = recipients::table
            .filter(recipients::id.eq(id))
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

    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<DomainRecipient>> {
        use crate::schema::recipients;
        let mut conn = self.pool.get()?;

        // Load recipients for the hub
        let db_recipients: Vec<Recipient> = recipients::table
            .filter(recipients::hub_id.eq(hub_id))
            .select(Recipient::as_select())
            .order(recipients::name.desc())
            .load(&mut conn)?;

        if db_recipients.is_empty() {
            return Ok(Vec::new());
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
                groups: recipient_id_to_group_ids.remove(&r.id).unwrap_or_default(),
            })
            .collect();

        Ok(recipients)
    }

    fn list_custom_fields(&self, hub_id: i32) -> RepositoryResult<Vec<String>> {
        use crate::schema::{recipient_fields, recipients};

        let mut conn = self.pool.get()?;

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

impl RecipientWriter for DieselRecipientRepository<'_> {
    fn create(&self, recipient: &[DomainNewRecipient]) -> RepositoryResult<usize> {
        use crate::schema::{groups, groups_recipients, recipient_fields, recipients};

        let mut conn = self.pool.get()?;

        conn.transaction::<usize, RepositoryError, _>(|conn| {
            let mut count_inserted: usize = 0;

            for new in recipient {
                let db_new = NewRecipient {
                    name: &new.name,
                    email: &new.email,
                    hub_id: new.hub_id,
                };

                let inserted = diesel::insert_into(recipients::table)
                    .values(&db_new)
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
                        diesel::insert_into(recipient_fields::table)
                            .values(&new_fields)
                            .execute(conn)?;
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

    fn update(
        &self,
        id: i32,
        recipient: &DomainUpdateRecipient,
    ) -> RepositoryResult<DomainRecipient> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.pool.get()?;

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

    fn delete(&self, id: i32) -> RepositoryResult<()> {
        use crate::schema::recipients;
        let mut conn = self.pool.get()?;
        diesel::delete(recipients::table.filter(recipients::id.eq(id))).execute(&mut conn)?;
        Ok(())
    }

    fn delete_all(&self, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::{groups_recipients, recipient_fields, recipients};
        let mut conn = self.pool.get()?;

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
