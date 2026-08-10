use std::collections::HashMap;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdRequest")]
pub struct Request {
    pub session_id: String,
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse")]
pub struct Response {
    /// Map to get the owner task id for each result id.
    #[armonik(
        with = "crate::codec::adapters::PairMap",
        absorbs = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse.MapResultTask"
    )]
    pub result_task: HashMap<String, String>,
    pub session_id: String,
}
