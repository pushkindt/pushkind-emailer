//! Service layer implementing business logic over repositories.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use pushkind_common::services::errors::{ServiceError, ServiceResult};

use crate::{SERVICE_ACCESS_ROLE, SERVICE_ADMIN_ROLE};

pub mod api;
pub mod groups;
pub mod main;
pub mod recipients;
pub mod settings;

fn ensure_emailer(user: &AuthenticatedUser) -> ServiceResult<()> {
    ensure_role(user, SERVICE_ACCESS_ROLE)
}

fn ensure_admin(user: &AuthenticatedUser) -> ServiceResult<()> {
    ensure_role(user, SERVICE_ADMIN_ROLE)
}

fn ensure_role(user: &AuthenticatedUser, role: &str) -> ServiceResult<()> {
    if check_role(role, &user.roles) {
        Ok(())
    } else {
        Err(ServiceError::Unauthorized)
    }
}
