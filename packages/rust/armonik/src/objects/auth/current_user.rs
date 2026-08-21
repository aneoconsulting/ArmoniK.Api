use super::User;

#[armonik_macros::message("armonik.api.grpc.v1.auth.GetCurrentUserRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

#[armonik_macros::message("armonik.api.grpc.v1.auth.GetCurrentUserResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub user: User,
}
