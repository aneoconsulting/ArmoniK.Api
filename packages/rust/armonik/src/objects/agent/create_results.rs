use super::ResultMetaData;

/// Result to create with data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsRequest.ResultCreate")]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The data associated to the result to create.
    pub data: bytes::Bytes,
}

impl<K: Into<String>, V: Into<bytes::Bytes>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
        }
    }
}

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsRequest")]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsResponse")]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: Vec<ResultMetaData>,
}
