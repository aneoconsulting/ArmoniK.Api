/// Request deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.DeleteResultsDataRequest")]
pub struct Request {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the results to delete.
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}

/// Response deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.DeleteResultsDataResponse")]
pub struct Response {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the deleted results.
    #[armonik(rename = "result_id")]
    pub result_ids: Vec<String>,
}
