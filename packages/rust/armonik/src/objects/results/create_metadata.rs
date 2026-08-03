use super::Raw;

/// Result to create without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest.ResultCreate")]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The session in which create results.
    pub manual_deletion: bool,
}

impl<T: Into<String>> From<T> for RequestItem {
    fn from(value: T) -> Self {
        Self {
            name: value.into(),
            manual_deletion: false,
        }
    }
}

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataRequest")]
pub struct Request {
    /// Results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.CreateResultsMetaDataResponse")]
pub struct Response {
    /// The list of raw results that were created.
    pub results: Vec<Raw>,
}
