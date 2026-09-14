#[armonik_macros::message("armonik.api.grpc.v1.Session")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Session {
    #[armonik(rename = "id")]
    pub session_id: String,
}
