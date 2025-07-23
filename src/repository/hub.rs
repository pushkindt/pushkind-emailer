use diesel::prelude::*;
use pushkind_common::db::DbPool;

use crate::domain::hub::{Hub, NewHub, UpdateHub};
use crate::models::hub::{Hub as DbHub, NewHub as DbNewHub, UpdateHub as DbUpdateHub};
use crate::repository::errors::RepositoryResult;
use crate::repository::{HubReader, HubWriter};

/// Diesel implementation of [`HubRepository`].
pub struct DieselHubRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselHubRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl HubReader for DieselHubRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Hub>> {
        use crate::schema::hubs;
        let mut conn = self.pool.get()?;
        let result = hubs::table
            .filter(hubs::id.eq(id))
            .first::<DbHub>(&mut conn)
            .optional()?;
        Ok(result.map(Into::into))
    }

    fn list(&self) -> RepositoryResult<Vec<Hub>> {
        use crate::schema::hubs;
        let mut conn = self.pool.get()?;
        let result = hubs::table.load::<DbHub>(&mut conn)?;
        Ok(result.into_iter().map(Into::into).collect())
    }
}

impl HubWriter for DieselHubRepository<'_> {
    fn create(&self, hub: &NewHub) -> RepositoryResult<Hub> {
        use crate::schema::hubs;
        let mut conn = self.pool.get()?;
        let result = diesel::insert_into(hubs::table)
            .values(DbNewHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(result.into())
    }

    fn update(&self, id: i32, hub: &UpdateHub) -> RepositoryResult<Hub> {
        use crate::schema::hubs;
        let mut conn = self.pool.get()?;
        let result = diesel::update(hubs::table.filter(hubs::id.eq(id)))
            .set(DbUpdateHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(result.into())
    }
}
