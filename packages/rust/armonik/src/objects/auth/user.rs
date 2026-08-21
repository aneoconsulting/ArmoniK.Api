#[armonik_macros::message("armonik.api.grpc.v1.auth.User")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
