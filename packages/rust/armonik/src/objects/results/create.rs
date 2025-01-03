use std::collections::HashMap;

use super::Raw;

use crate::api::v3;

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Results to create.
    pub results: HashMap<String, Vec<u8>>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<Request> for v3::results::CreateResultsRequest {
    fn from(value: Request) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(
                    |(name, data)| v3::results::create_results_request::ResultCreate { name, data },
                )
                .collect(),
            session_id: value.session_id,
        }
    }
}

impl From<v3::results::CreateResultsRequest> for Request {
    fn from(value: v3::results::CreateResultsRequest) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|result| (result.name, result.data))
                .collect(),
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(req Request : v3::results::CreateResultsRequest);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The list of raw results that were created.
    pub results: HashMap<String, Raw>,
}

impl From<Response> for v3::results::CreateResultsResponse {
    fn from(value: Response) -> Self {
        Self {
            results: value.results.into_values().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::CreateResultsResponse> for Response {
    fn from(value: v3::results::CreateResultsResponse) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|result| (result.name.clone(), result.into()))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::results::CreateResultsResponse);
