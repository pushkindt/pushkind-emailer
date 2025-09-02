use diesel::prelude::*;
use pushkind_common::domain::emailer::hub::{
    Hub as DomainHub, NewHub as DomainNewHub, UpdateHub as DomainUpdateHub,
};
use pushkind_common::models::emailer::hub::{
    Hub as DbHub, NewHub as DbNewHub, UpdateHub as DbUpdateHub,
};
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::{DieselRepository, HubReader, HubWriter};

impl HubReader for DieselRepository {
    fn get_hub_by_id(&self, id: i32) -> RepositoryResult<Option<DomainHub>> {
        use pushkind_common::schema::emailer::hubs;
        let mut conn = self.conn()?;
        let result = hubs::table
            .filter(hubs::id.eq(id))
            .first::<DbHub>(&mut conn)
            .optional()?;
        Ok(result.map(Into::into))
    }
}

impl HubWriter for DieselRepository {
    fn create_hub(&self, hub: &DomainNewHub) -> RepositoryResult<DomainHub> {
        use pushkind_common::schema::emailer::hubs;
        let mut conn = self.conn()?;
        let result = diesel::insert_into(hubs::table)
            .values(DbNewHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(result.into())
    }

    fn update_hub(&self, id: i32, hub: &DomainUpdateHub) -> RepositoryResult<DomainHub> {
        use pushkind_common::schema::emailer::hubs;
        let mut conn = self.conn()?;
        let result = diesel::update(hubs::table.filter(hubs::id.eq(id)))
            .set(DbUpdateHub::from(hub))
            .get_result::<DbHub>(&mut conn)?;
        Ok(result.into())
    }
}
