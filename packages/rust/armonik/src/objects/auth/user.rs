use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

super::super::impl_convert!(struct User = v3::auth::User { username, list roles, list permissions  });
