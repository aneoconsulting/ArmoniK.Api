#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.ResultRequest")]
pub struct ResultRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}
