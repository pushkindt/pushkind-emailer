/// Diesel implementation of [`GroupRepository`].
pub struct DieselGroupRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> DieselGroupRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl GroupReader for DieselGroupRepository<'_> {}
impl GroupWriter for DieselGroupRepository<'_> {}
