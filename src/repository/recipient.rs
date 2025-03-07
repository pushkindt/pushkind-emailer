use diesel::prelude::*;

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

    // Convert HashMap values into Vec<(Group, Vec<Recipient>)>
    Ok(groups_map.into_values().collect())
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
