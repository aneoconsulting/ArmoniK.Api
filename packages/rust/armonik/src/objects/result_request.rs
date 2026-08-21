#[armonik_macros::message("armonik.api.grpc.v1.ResultRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}
