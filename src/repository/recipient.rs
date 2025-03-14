use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use diesel::result::Error;
use log::info;
use serde::Deserialize;

use crate::{
    forms::{
        groups::{AddGroupForm, AssignGroupRecipientForm},
        recipients::{AddRecipientForm, SaveRecipientForm},
    },
    models::recipient::{Group, GroupRecipient, NewGroup, NewRecipient, Recipient, RecipientField},
};

pub fn get_hub_all_recipients(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<Vec<(Recipient, HashMap<String, String>, Vec<Group>)>> {
    use crate::schema::{groups, recipients};

    // Load recipients
    let recipients = recipients::table
        .filter(recipients::hub_id.eq(hub))
        .select(Recipient::as_select())
        .order(recipients::name.desc())
        .load::<Recipient>(conn)?;

    // Load recipient fields (key-value pairs)
    let recipient_fields = RecipientField::belonging_to(&recipients)
        .select(RecipientField::as_select())
        .load::<RecipientField>(conn)?
        .grouped_by(&recipients);

    // Load GroupRecipient entries for these recipients
    let group_recipients = GroupRecipient::belonging_to(&recipients)
        .select(GroupRecipient::as_select())
        .load::<GroupRecipient>(conn)?;

    // Extract unique group IDs to avoid redundant queries
    let group_ids: HashSet<i32> = group_recipients.iter().map(|gr| gr.group_id).collect();

    // Load groups based on the extracted unique group IDs
    let groups = groups::table
        .filter(groups::id.eq_any(&group_ids))
        .select(Group::as_select())
        .load::<Group>(conn)?;

    // Create a HashMap for quick lookup of Group by group_id
    let group_map: HashMap<i32, Group> = groups.into_iter().map(|g| (g.id, g)).collect();

    // Group the groups by recipient_id using HashMap
    let mut recipient_groups: HashMap<i32, Vec<Group>> = HashMap::new();
    for gr in group_recipients {
        if let Some(group) = group_map.get(&gr.group_id) {
            recipient_groups
                .entry(gr.recipient_id)
                .or_insert_with(Vec::new)
                .push(group.clone());
        }
    }

    // Combine everything into the expected structure
    Ok(recipients
        .into_iter()
        .zip(recipient_fields.into_iter())
        .map(|(recipient, fields)| {
            let field_map = fields.into_iter().map(|rf| (rf.field, rf.value)).collect();
            let groups = recipient_groups
                .remove(&recipient.id)
                .unwrap_or_else(Vec::new);
            (recipient, field_map, groups)
        })
        .collect())
}

pub fn get_hub_group_recipients(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<Vec<(Group, Vec<Recipient>)>> {
    use crate::schema::groups;
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    // Perform a join from groups -> groups_recipients -> recipients
    let results = groups::table
        .filter(groups::hub_id.eq(hub))
        .left_join(groups_recipients::table.on(groups::id.eq(groups_recipients::group_id)))
        .left_join(recipients::table.on(groups_recipients::recipient_id.eq(recipients::id)))
        .select((Group::as_select(), Option::<Recipient>::as_select())) // Allow for empty recipients
        .load::<(Group, Option<Recipient>)>(conn)?;

    // Use a HashMap to aggregate recipients by group
    use std::collections::HashMap;

    let mut groups_map: HashMap<i32, (Group, Vec<Recipient>)> = HashMap::new();

    for (group, recipient_opt) in results {
        let entry = groups_map
            .entry(group.id)
            .or_insert_with(|| (group, Vec::new()));

        // If there's a recipient, add it to the vector
        if let Some(recipient) = recipient_opt {
            entry.1.push(recipient);
        }
    }

    // Convert HashMap values into Vec<(Group, Vec<Recipient>)> sorted by Group.name
    let mut result: Vec<(Group, Vec<Recipient>)> = groups_map
        .into_values()
        .map(|(group, recipients)| {
            let mut recipients = recipients;
            recipients.sort_by(|a, b| a.name.cmp(&b.name));
            (group, recipients)
        })
        .collect();
    result.sort_by(|(a, _), (b, _)| a.name.cmp(&b.name));
    Ok(result)
}

pub fn create_recipient(
    conn: &mut SqliteConnection,
    hub: i32,
    recipient: &AddRecipientForm,
) -> QueryResult<usize> {
    use crate::schema::recipients;

    let new_recipient = NewRecipient {
        hub_id: hub,
        name: &recipient.name,
        email: &recipient.email,
    };

    diesel::insert_into(recipients::table)
        .values(new_recipient)
        .execute(conn)
}

pub fn delete_recipient(conn: &mut SqliteConnection, recipient: i32) -> QueryResult<usize> {
    use crate::schema::recipients;

    diesel::delete(recipients::table.filter(recipients::id.eq(recipient))).execute(conn)
}

pub fn create_group(
    conn: &mut SqliteConnection,
    hub: i32,
    group: &AddGroupForm,
) -> QueryResult<usize> {
    use crate::schema::groups;

    let new_group = NewGroup {
        hub_id: hub,
        name: &group.name,
    };

    diesel::insert_into(groups::table)
        .values(new_group)
        .execute(conn)
}

pub fn delete_group(conn: &mut SqliteConnection, group: i32) -> QueryResult<usize> {
    use crate::schema::groups;

    diesel::delete(groups::table.filter(groups::id.eq(group))).execute(conn)
}

pub fn assign_recipient_to_group(
    conn: &mut SqliteConnection,
    form: &AssignGroupRecipientForm,
) -> QueryResult<usize> {
    use crate::schema::groups_recipients;

    let new_assignment = GroupRecipient {
        recipient_id: form.recipient_id,
        group_id: form.group_id,
    };

    diesel::insert_into(groups_recipients::table)
        .values(new_assignment)
        .execute(conn)
}

pub fn unassign_recipient_from_group(
    conn: &mut SqliteConnection,
    form: &AssignGroupRecipientForm,
) -> QueryResult<usize> {
    use crate::schema::groups_recipients;

    diesel::delete(
        groups_recipients::table.filter(
            groups_recipients::recipient_id
                .eq(form.recipient_id)
                .and(groups_recipients::group_id.eq(form.group_id)),
        ),
    )
    .execute(conn)
}

pub fn clean_all_recipients_and_groups(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<usize> {
    use crate::schema::groups;
    use crate::schema::groups_recipients;
    use crate::schema::recipient_fields;
    use crate::schema::recipients;

    // get all recipient ids
    let recipient_ids = recipients::table
        .filter(recipients::hub_id.eq(hub))
        .select(recipients::id)
        .load::<i32>(conn)?;

    diesel::delete(recipients::table.filter(recipients::hub_id.eq(hub))).execute(conn)?;
    diesel::delete(groups::table.filter(groups::hub_id.eq(hub))).execute(conn)?;

    // delete all recipient fields
    diesel::delete(
        recipient_fields::table.filter(recipient_fields::recipient_id.eq_any(&recipient_ids)),
    )
    .execute(conn)?;

    // delete all groups_recipients
    diesel::delete(
        groups_recipients::table.filter(groups_recipients::recipient_id.eq_any(&recipient_ids)),
    )
    .execute(conn)
}

#[derive(Debug, Deserialize)]
struct RecipientCSV {
    name: String,
    email: String,
    groups: Vec<String>,
    optional_fields: HashMap<String, String>,
}

fn parse_recipients_csv(csv: &str) -> Result<Vec<RecipientCSV>, Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_reader(csv.as_bytes());

    let headers = rdr.headers()?.clone();
    let mut recipients = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let mut optional_fields = HashMap::new();

        let mut name = String::new();
        let mut email = String::new();
        let mut groups = Vec::new();

        for (i, field) in record.iter().enumerate() {
            match headers.get(i) {
                Some("name") => name = field.to_string(),
                Some("email") => email = field.to_string(),
                Some("groups") => {
                    groups = field
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                Some(header) => {
                    if field.len() == 0 {
                        continue;
                    }
                    optional_fields.insert(header.to_string(), field.to_string());
                }
                None => continue,
            }
        }

        recipients.push(RecipientCSV {
            name,
            email,
            groups,
            optional_fields,
        });
    }

    Ok(recipients)
}

pub fn update_recipients_from_csv(
    conn: &mut SqliteConnection,
    hub_id: i32,
    csv: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let recipients = parse_recipients_csv(csv)?;

    for recipient in recipients {
        conn.transaction::<_, Error, _>(|conn| {
            insert_or_update_recipient_with_groups(conn, hub_id, recipient)
        })?;
    }
    Ok(())
}

fn get_or_insert_group(
    conn: &mut SqliteConnection,
    group_name: &str,
    hub_id: i32,
) -> Result<i32, Error> {
    use crate::schema::groups;

    if let Some(group_id) = groups::table
        .filter(groups::name.eq(group_name))
        .select(groups::id)
        .first::<i32>(conn)
        .optional()?
    {
        return Ok(group_id);
    }

    let new_group = NewGroup {
        name: group_name,
        hub_id,
    };

    diesel::insert_into(groups::table)
        .values(&new_group)
        .execute(conn)?;

    groups::table
        .filter(groups::name.eq(group_name))
        .filter(groups::hub_id.eq(hub_id))
        .select(groups::id)
        .first::<i32>(conn)
}

fn insert_or_update_recipient_with_groups(
    conn: &mut SqliteConnection,
    hub_id: i32,
    csv: RecipientCSV,
) -> Result<(), Error> {
    use crate::schema::groups_recipients;
    use crate::schema::recipient_fields;
    use crate::schema::recipients;

    let existing_recipient = recipients::table
        .filter(recipients::email.eq(&csv.email))
        .select((recipients::id, recipients::name))
        .first::<(i32, String)>(conn)
        .optional()?;

    let recipient_id = match existing_recipient {
        Some((existing_id, existing_name)) => {
            if existing_name != csv.name {
                diesel::update(recipients::table.filter(recipients::id.eq(existing_id)))
                    .set(recipients::name.eq(&csv.name))
                    .execute(conn)?;
            }
            existing_id
        }
        None => {
            let new_recipient = NewRecipient {
                name: &csv.name,
                email: &csv.email,
                hub_id,
            };

            diesel::insert_into(recipients::table)
                .values(&new_recipient)
                .execute(conn)?;

            recipients::table
                .filter(recipients::email.eq(&csv.email))
                .select(recipients::id)
                .first::<i32>(conn)?
        }
    };

    diesel::delete(
        groups_recipients::table.filter(groups_recipients::recipient_id.eq(recipient_id)),
    )
    .execute(conn)?;

    for group_name in &csv.groups {
        let group_id = get_or_insert_group(conn, group_name, hub_id)?;

        // Insert new group-recipient relation
        let new_group_recipient = GroupRecipient {
            group_id,
            recipient_id,
        };

        diesel::insert_into(groups_recipients::table)
            .values(&new_group_recipient)
            .execute(conn)?;
    }

    diesel::delete(recipient_fields::table.filter(recipient_fields::recipient_id.eq(recipient_id)))
        .execute(conn)?;

    for (field, value) in csv.optional_fields {
        let new_field = RecipientField {
            recipient_id,
            field,
            value,
        };

        diesel::insert_into(recipient_fields::table)
            .values(&new_field)
            .execute(conn)?;
    }

    Ok(())
}

pub fn get_hub_all_groups(conn: &mut SqliteConnection, hub: i32) -> QueryResult<Vec<Group>> {
    use crate::schema::groups;

    groups::table
        .filter(groups::hub_id.eq(hub))
        .select(Group::as_select())
        .order(groups::name.desc())
        .load::<Group>(conn)
}

pub fn get_hub_all_recipients_fields(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<Vec<String>> {
    use crate::schema::recipient_fields;
    use crate::schema::recipients;

    recipients::table
        .filter(recipients::hub_id.eq(hub))
        .inner_join(recipient_fields::table.on(recipients::id.eq(recipient_fields::recipient_id)))
        .select(recipient_fields::field)
        .distinct()
        .load::<String>(conn)
}

pub fn get_recipient(conn: &mut SqliteConnection, recipient_id: i32) -> QueryResult<Recipient> {
    use crate::schema::recipients;

    recipients::table
        .filter(recipients::id.eq(recipient_id))
        .first::<Recipient>(conn)
}

pub fn get_recipient_fields(
    conn: &mut SqliteConnection,
    recipient_id: i32,
) -> QueryResult<Vec<RecipientField>> {
    use crate::schema::recipient_fields;

    recipient_fields::table
        .filter(recipient_fields::recipient_id.eq(recipient_id))
        .load::<RecipientField>(conn)
}

pub fn get_recipient_group_ids(
    conn: &mut SqliteConnection,
    recipient_id: i32,
) -> QueryResult<HashSet<i32>> {
    use crate::schema::groups_recipients;

    let result = groups_recipients::table
        .filter(groups_recipients::recipient_id.eq(recipient_id))
        .select(groups_recipients::group_id)
        .load::<i32>(conn)?
        .into_iter()
        .collect::<HashSet<i32>>();
    Ok(result)
}

pub fn save_recipient(
    conn: &mut SqliteConnection,
    recipient: &SaveRecipientForm,
) -> QueryResult<usize> {
    use crate::schema::groups;
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    diesel::update(recipients::table.filter(recipients::id.eq(recipient.id)))
        .set((
            recipients::name.eq(&recipient.name),
            recipients::email.eq(&recipient.email),
        ))
        .execute(conn)?;

    let groups = groups::table
        .filter(groups::id.eq_any(&recipient.groups))
        .select(groups::id)
        .load::<i32>(conn)?;

    diesel::delete(
        groups_recipients::table.filter(groups_recipients::recipient_id.eq(recipient.id)),
    )
    .execute(conn)?;

    diesel::insert_into(groups_recipients::table)
        .values(
            groups
                .iter()
                .map(|group_id| GroupRecipient {
                    recipient_id: recipient.id,
                    group_id: *group_id,
                })
                .collect::<Vec<GroupRecipient>>(),
        )
        .execute(conn)
}
