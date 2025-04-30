use crate::models::user::UserDto;

#[derive(Debug, thiserror::Error)]
pub enum UserRepoError {
    #[error("User not found")]
    NotFound,
    #[error("Invalid password or hashing error: {0}")]
    HashError(String),
    #[error("Unexpected error: {0}")]
    Other(String),
}

pub trait UserRepository {
    fn create_user(&self, email: &str, password: &str) -> Result<UserDto, UserRepoError>;
    fn find_user_by_email(&self, email: &str) -> Result<UserDto, UserRepoError>;
    fn get_user(&self, user_id: i32) -> Result<UserDto, UserRepoError>;
    fn set_user_hub(&self, user_id: i32, hub_id: Option<i32>) -> Result<usize, UserRepoError>;
}
