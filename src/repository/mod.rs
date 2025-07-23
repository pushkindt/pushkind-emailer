use crate::domain::hub::{Hub, NewHub, UpdateHub};
use crate::repository::errors::RepositoryResult;

pub mod email;
pub mod errors;
pub mod hub;
pub mod recipient;

pub trait EmailReader {}

pub trait EmailWriter {}

pub trait HubReader {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Hub>>;
    fn list(&self) -> RepositoryResult<Vec<Hub>>;
}

pub trait HubWriter {
    fn create(&self, hub: &NewHub) -> RepositoryResult<Hub>;
    fn update(&self, id: i32, hub: &UpdateHub) -> RepositoryResult<Hub>;
}

pub trait RecipientReader {}

pub trait RecipientWriter {}
