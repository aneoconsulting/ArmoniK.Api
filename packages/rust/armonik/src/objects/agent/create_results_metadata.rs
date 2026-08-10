use super::ResultMetaData;

/// Result to create without data.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsMetaDataRequest")]
pub struct Request {
    pub communication_token: String,
    pub session_id: String,
    /// The list of names for the results to create.
    pub results: Vec<RequestItem>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateResultsMetaDataResponse")]
pub struct Response {
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: Vec<ResultMetaData>,
}
