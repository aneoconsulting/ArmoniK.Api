use super::Raw;

/// Request to get an result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.GetResultRequest")]
pub struct Request {
    /// Result id. Must fail when name is empty.
    #[armonik(rename = "result_id")]
    pub id: String,
}

/// Response to get an result.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.GetResultResponse")]
pub struct Response {
    /// The result.
    pub result: Raw,
}
