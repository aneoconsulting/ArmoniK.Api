#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.auth.User")]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}
