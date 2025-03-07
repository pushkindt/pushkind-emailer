use diesel::prelude::*;

use crate::{
    forms::recipients::AddRecipientForm,
    models::recipient::{Group, NewGroup, NewRecipient, Recipient},
};

pub fn get_hub_recipients(conn: &mut SqliteConnection, hub: i32) -> QueryResult<Vec<Recipient>> {
    use crate::schema::recipients::dsl::*;

    recipients.filter(hub_id.eq(hub)).load(conn)
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
    use crate::schema::recipients::dsl::*;

    diesel::delete(recipients.filter(id.eq(recipient))).execute(conn)
}
