//! Repository operations for hubs.
use crate::domain::hub::{Hub as DomainHub, NewHub as DomainNewHub, UpdateHub as DomainUpdateHub};
use crate::models::hub::{Hub as DbHub, NewHub as DbNewHub, UpdateHub as DbUpdateHub};
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::repository::{DieselRepository, HubReader, HubWriter};

impl HubReader for DieselRepository {
    fn get_hub_by_id(&self, id: i32) -> RepositoryResult<Option<DomainHub>> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = hubs::table
            .filter(hubs::id.eq(id))
            .first::<DbHub>(&mut conn)
            .optional()?;
        result
            .map(DomainHub::try_from)
            .transpose()
            .map_err(|err| RepositoryError::ValidationError(err.to_string()))
    }
}

impl HubWriter for DieselRepository {
    fn create_hub(&self, hub: &DomainNewHub) -> RepositoryResult<DomainHub> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = diesel::insert_into(hubs::table)
            .values(DbNewHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        DomainHub::try_from(result).map_err(|err| RepositoryError::ValidationError(err.to_string()))
    }

    fn update_hub(&self, id: i32, hub: &DomainUpdateHub) -> RepositoryResult<DomainHub> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = diesel::update(hubs::table.filter(hubs::id.eq(id)))
            .set(DbUpdateHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        DomainHub::try_from(result).map_err(|err| RepositoryError::ValidationError(err.to_string()))
    }
}
