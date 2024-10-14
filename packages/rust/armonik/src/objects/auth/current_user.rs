use super::User;

use crate::api::v3;

/// Request to get current user information.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

super::super::impl_convert!(struct Request = v3::auth::GetCurrentUserRequest {});

/// Response to get current user information.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Return current user. If auth failed, must throw a gRPC error.
    pub user: User,
}

super::super::impl_convert!(struct Response = v3::auth::GetCurrentUserResponse { user = option user });
