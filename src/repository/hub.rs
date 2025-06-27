use diesel::prelude::*;

use crate::models::hub::Hub;

pub fn update_hub(conn: &mut SqliteConnection, hub: &Hub) -> QueryResult<usize> {
    use crate::schema::hubs::dsl::hubs;

    diesel::update(hubs).set(hub).execute(conn)
}

pub fn get_hub(conn: &mut SqliteConnection, hub_id: i32) -> QueryResult<Hub> {
    use crate::schema::hubs::dsl::{hubs, id};

    hubs.filter(id.eq(hub_id)).first(conn)
}
