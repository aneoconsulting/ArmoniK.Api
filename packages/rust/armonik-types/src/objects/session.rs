#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Session")]
pub struct Session {
    #[armonik(rename = "id")]
    pub session_id: String,
}
