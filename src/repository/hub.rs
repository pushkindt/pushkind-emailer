//! Repository operations for hubs.
use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::hub::{Hub as DomainHub, NewHub as DomainNewHub, UpdateHub as DomainUpdateHub};
use crate::domain::types::HubId;
use crate::models::hub::{Hub as DbHub, NewHub as DbNewHub, UpdateHub as DbUpdateHub};
use crate::repository::{DieselRepository, HubReader, HubWriter};

impl HubReader for DieselRepository {
    fn get_hub_by_id(&self, id: HubId) -> RepositoryResult<Option<DomainHub>> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = hubs::table
            .filter(hubs::id.eq(id.get()))
            .first::<DbHub>(&mut conn)
            .optional()?;
        Ok(result.map(DomainHub::try_from).transpose()?)
    }
}

impl HubWriter for DieselRepository {
    fn create_hub(&self, hub: &DomainNewHub) -> RepositoryResult<DomainHub> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = diesel::insert_into(hubs::table)
            .values(DbNewHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(DomainHub::try_from(result)?)
    }

    fn update_hub(&self, id: HubId, hub: &DomainUpdateHub) -> RepositoryResult<DomainHub> {
        use crate::schema::hubs;
        let mut conn = self.conn()?;
        let result = diesel::update(hubs::table.filter(hubs::id.eq(id.get())))
            .set(DbUpdateHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(DomainHub::try_from(result)?)
    }
}
