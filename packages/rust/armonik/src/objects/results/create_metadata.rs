use std::collections::{HashMap, HashSet};

use super::Raw;

use crate::api::v3;

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The list of results to create.
    pub results: HashSet<String>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<Request> for v3::results::CreateResultsMetaDataRequest {
    fn from(value: Request) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(
                    |result| v3::results::create_results_meta_data_request::ResultCreate {
                        name: result,
                    },
                )
                .collect(),
            session_id: value.session_id,
        }
    }
}

impl From<v3::results::CreateResultsMetaDataRequest> for Request {
    fn from(value: v3::results::CreateResultsMetaDataRequest) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|result| result.name)
                .collect(),
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(req Request : v3::results::CreateResultsMetaDataRequest);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The list of raw results that were created.
    pub results: HashMap<String, Raw>,
}

impl From<Response> for v3::results::CreateResultsMetaDataResponse {
    fn from(value: Response) -> Self {
        Self {
            results: value.results.into_values().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::CreateResultsMetaDataResponse> for Response {
    fn from(value: v3::results::CreateResultsMetaDataResponse) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|result| (result.name.clone(), result.into()))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::results::CreateResultsMetaDataResponse);