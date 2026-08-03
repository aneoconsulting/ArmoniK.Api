use super::Raw;

/// Result to create with data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsRequest.ResultCreate")]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The data associated to the result to create.
    pub data: bytes::Bytes,
    /// The session in which create results.
    pub manual_deletion: bool,
}

impl<K: Into<String>, V: Into<bytes::Bytes>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            manual_deletion: false,
        }
    }
}

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsRequest")]
pub struct Request {
    /// The session in which create results.
    pub session_id: String,
    /// Results to create.
    pub results: Vec<RequestItem>,
}

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsResponse")]
pub struct Response {
    /// The list of raw results that were created.
    pub results: Vec<Raw>,
}
