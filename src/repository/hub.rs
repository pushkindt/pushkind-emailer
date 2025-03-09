use diesel::prelude::*;

use crate::{
    models::hub::{Hub, NewHub},
    repository::user::set_user_hub,
};

pub fn create_hub(conn: &mut SqliteConnection, name: &str) -> QueryResult<Hub> {
    use crate::schema::hubs::dsl::{hubs, id};

    let new_hub = NewHub { name };

    diesel::insert_into(hubs).values(&new_hub).execute(conn)?;

    hubs.order(id.desc()).first(conn)
}

pub fn list_hubs(conn: &mut SqliteConnection) -> QueryResult<Vec<Hub>> {
    use crate::schema::hubs::dsl::hubs;

    hubs.load(conn)
}

pub fn update_hub(conn: &mut SqliteConnection, hub: &Hub) -> QueryResult<usize> {
    use crate::schema::hubs::dsl::hubs;

    diesel::update(hubs).set(hub).execute(conn)
}

pub fn get_hub(conn: &mut SqliteConnection, hub_id: i32) -> QueryResult<Hub> {
    use crate::schema::hubs::dsl::{hubs, id};

    hubs.filter(id.eq(hub_id)).first(conn)
}

pub fn delete_hub(conn: &mut SqliteConnection, user_id: i32, hub_id: i32) -> QueryResult<usize> {
    use crate::schema::hubs;

    diesel::delete(hubs::table.filter(hubs::id.eq(hub_id))).execute(conn)?;

    // set user hub to null
    set_user_hub(conn, user_id, None)
}
