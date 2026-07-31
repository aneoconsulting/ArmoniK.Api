use std::collections::HashMap;

/// Request for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdRequest")]
pub struct Request {
    /// The session ID.
    pub session_id: String,
    /// The list of result ID/name.
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}

/// Response for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse")]
pub struct Response {
    /// Map to get the owner task id for each result id.
    #[armonik(
        with = "crate::codec::adapters::PairMap<1, 2>",
        absorbs = "armonik.api.grpc.v1.results.GetOwnerTaskIdResponse.MapResultTask"
    )]
    pub result_task: HashMap<String, String>,
    /// The session ID.
    pub session_id: String,
}
