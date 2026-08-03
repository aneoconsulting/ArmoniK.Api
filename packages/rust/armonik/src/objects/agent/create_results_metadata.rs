use super::ResultMetaData;

/// Result to create without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsMetaDataRequest.ResultCreate")]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
}

impl<T: Into<String>> From<T> for RequestItem {
    fn from(value: T) -> Self {
        Self { name: value.into() }
    }
}

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsMetaDataRequest")]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The session in which create results.
    pub session_id: String,
    /// The list of names for the results to create.
    pub results: Vec<RequestItem>,
}

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsMetaDataResponse")]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: Vec<ResultMetaData>,
}
