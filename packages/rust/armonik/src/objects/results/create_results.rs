use std::collections::HashMap;

use super::ResultRaw;

use crate::api::v3;

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreateResultsRequest {
    /// Results to create.
    pub results: HashMap<String, Vec<u8>>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<CreateResultsRequest> for v3::results::CreateResultsRequest {
    fn from(value: CreateResultsRequest) -> Self {
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

impl From<v3::results::CreateResultsRequest> for CreateResultsRequest {
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

super::super::impl_convert!(CreateResultsRequest : Option<v3::results::CreateResultsRequest>);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
pub struct CreateResultsResponse {
    /// The list of raw results that were created.
    pub results: Vec<ResultRaw>,
}

impl From<CreateResultsResponse> for v3::results::CreateResultsResponse {
    fn from(value: CreateResultsResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::CreateResultsResponse> for CreateResultsResponse {
    fn from(value: v3::results::CreateResultsResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(CreateResultsResponse : Option<v3::results::CreateResultsResponse>);
