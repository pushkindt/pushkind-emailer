use diesel::prelude::*;
use diesel::result::Error;
use serde::Deserialize;

use crate::{
    forms::recipients::{AddGroupForm, AddRecipientForm, AssignGroupRecipientForm},
    models::recipient::{Group, GroupRecipient, NewGroup, NewRecipient, Recipient},
};

pub fn get_hub_all_recipients(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<Vec<Recipient>> {
    use crate::schema::recipients;

    recipients::table
        .filter(recipients::hub_id.eq(hub))
        .select(Recipient::as_select())
        .order(recipients::name.desc())
        .load::<Recipient>(conn)
}

pub fn get_hub_nogroup_recipients(
    conn: &mut SqliteConnection,
    hub: i32,
) -> QueryResult<Vec<Recipient>> {
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    recipients::table
        .left_join(groups_recipients::table.on(recipients::id.eq(groups_recipients::recipient_id)))
        .filter(recipients::hub_id.eq(hub))
        .filter(groups_recipients::recipient_id.is_null()) // Filter for unassigned recipients
        .select(Recipient::as_select()) // Select all recipient fields
        .order(recipients::name.desc())
        .load::<Recipient>(conn)
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
    use crate::schema::recipients;

    diesel::delete(recipients::table.filter(recipients::hub_id.eq(hub))).execute(conn)?;
    diesel::delete(groups_recipients::table).execute(conn)?;
    diesel::delete(groups::table.filter(groups::hub_id.eq(hub))).execute(conn)
}

#[derive(Debug, Deserialize)]
struct RecipientCSV {
    name: String,
    email: String,
    groups: String, // comma separated group names
}

pub fn parse_recipients_csv(
    conn: &mut SqliteConnection,
    hub_id: i32,
    csv: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_reader(csv.as_bytes());
    for result in rdr.deserialize() {
        let record: RecipientCSV = result?;

        conn.transaction::<_, Error, _>(|conn| {
            insert_or_update_recipient_with_groups(conn, hub_id, record)
        })?;
    }
    Ok(())
}

fn insert_or_update_recipient_with_groups(
    conn: &mut SqliteConnection,
    hub_id: i32,
    csv: RecipientCSV,
) -> Result<(), Error> {
    use crate::schema::groups;
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    let existing_recipient = recipients::table
        .filter(recipients::email.eq(&csv.email))
        .select((recipients::id, recipients::name))
        .first::<(i32, String)>(conn)
        .optional()?;

    let recipient_id = if let Some((existing_id, existing_name)) = existing_recipient {
        // Update recipient's name if different
        if existing_name != csv.name {
            diesel::update(recipients::table.filter(recipients::id.eq(existing_id)))
                .set(recipients::name.eq(&csv.name))
                .execute(conn)?;
        }
        existing_id
    } else {
        // Insert new recipient and get its ID
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
    };

    let group_names: Vec<&str> = csv.groups.split(',').map(|s| s.trim()).collect();

    diesel::delete(
        groups_recipients::table.filter(groups_recipients::recipient_id.eq(recipient_id)),
    )
    .execute(conn)?;

    for group_name in group_names {
        if group_name.is_empty() {
            continue;
        }

        // Check if group exists or insert new one
        let group_id = groups::table
            .filter(groups::name.eq(group_name))
            .select(groups::id)
            .first::<i32>(conn)
            .optional()?
            .unwrap_or_else(|| {
                // Insert new group
                let new_group = NewGroup {
                    name: group_name,
                    hub_id,
                };
                diesel::insert_into(groups::table)
                    .values(&new_group)
                    .execute(conn)
                    .unwrap();

                // Get newly created group ID
                groups::table
                    .filter(groups::name.eq(group_name))
                    .filter(groups::hub_id.eq(hub_id))
                    .select(groups::id)
                    .first::<i32>(conn)
                    .unwrap()
            });

        // Insert new group-recipient relation
        let new_group_recipient = GroupRecipient {
            group_id,
            recipient_id,
        };

        diesel::insert_into(groups_recipients::table)
            .values(&new_group_recipient)
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
