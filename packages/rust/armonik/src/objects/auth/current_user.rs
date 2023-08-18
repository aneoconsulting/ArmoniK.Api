use super::User;

use crate::api::v3;

/// Request to get current user informations.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

impl From<Request> for v3::auth::GetCurrentUserRequest {
    fn from(_value: Request) -> Self {
        Self {}
    }
}

impl From<v3::auth::GetCurrentUserRequest> for Request {
    fn from(_value: v3::auth::GetCurrentUserRequest) -> Self {
        Self {}
    }
}

super::super::impl_convert!(Request : Option<v3::auth::GetCurrentUserRequest>);

/// Response to get current user informations.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Return current user. If auth failed, must throw a gRPC error.
    pub user: User,
}

impl From<Response> for v3::auth::GetCurrentUserResponse {
    fn from(value: Response) -> Self {
        Self {
            user: value.user.into(),
        }
    }
}

impl From<v3::auth::GetCurrentUserResponse> for Response {
    fn from(value: v3::auth::GetCurrentUserResponse) -> Self {
        Self {
            user: value.user.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::auth::GetCurrentUserResponse>);
