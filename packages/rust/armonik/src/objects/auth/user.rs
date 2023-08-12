use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl From<User> for v3::auth::User {
    fn from(value: User) -> Self {
        Self {
            username: value.username,
            roles: value.roles,
            permissions: value.permissions,
        }
    }
}

impl From<v3::auth::User> for User {
    fn from(value: v3::auth::User) -> Self {
        Self {
            username: value.username,
            roles: value.roles,
            permissions: value.permissions,
        }
    }
}

super::super::impl_convert!(User : Option<v3::auth::User>);

impl From<User> for v3::auth::GetCurrentUserResponse {
    fn from(value: User) -> Self {
        Self { user: value.into() }
    }
}

impl From<v3::auth::GetCurrentUserResponse> for User {
    fn from(value: v3::auth::GetCurrentUserResponse) -> Self {
        value.user.into()
    }
}

super::super::impl_convert!(User : Option<v3::auth::GetCurrentUserResponse>);
