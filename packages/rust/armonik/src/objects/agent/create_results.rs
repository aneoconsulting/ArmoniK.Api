use std::collections::HashMap;

use super::ResultMetaData;

use crate::api::v3;

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Results to create.
    pub results: HashMap<String, Vec<u8>>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<Request> for v3::agent::CreateResultsRequest {
    fn from(value: Request) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value
                .results
                .into_iter()
                .map(|(name, data)| v3::agent::create_results_request::ResultCreate { name, data })
                .collect(),
            session_id: value.session_id,
        }
    }
}

impl From<v3::agent::CreateResultsRequest> for Request {
    fn from(value: v3::agent::CreateResultsRequest) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value
                .results
                .into_iter()
                .map(|result| (result.name, result.data))
                .collect(),
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::CreateResultsRequest);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: HashMap<String, ResultMetaData>,
}

impl From<Response> for v3::agent::CreateResultsResponse {
    fn from(value: Response) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value.results.into_values().map(Into::into).collect(),
        }
    }
}

impl From<v3::agent::CreateResultsResponse> for Response {
    fn from(value: v3::agent::CreateResultsResponse) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value
                .results
                .into_iter()
                .map(|result| (result.name.clone(), result.into()))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::agent::CreateResultsResponse);
