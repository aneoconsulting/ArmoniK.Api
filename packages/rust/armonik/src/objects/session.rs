#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[armonik(message = "armonik.api.grpc.v1.Session")]
pub struct Session {
    #[armonik(rename = "id")]
    pub session_id: String,
}
