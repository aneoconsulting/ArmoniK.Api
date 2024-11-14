use std::collections::HashMap;

use super::ResultMetaData;

use crate::api::v3;

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of names for the results to create.
    pub names: Vec<String>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<Request> for v3::agent::CreateResultsMetaDataRequest {
    fn from(value: Request) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value
                .names
                .into_iter()
                .map(
                    |result| v3::agent::create_results_meta_data_request::ResultCreate {
                        name: result,
                    },
                )
                .collect(),
            session_id: value.session_id,
        }
    }
}

impl From<v3::agent::CreateResultsMetaDataRequest> for Request {
    fn from(value: v3::agent::CreateResultsMetaDataRequest) -> Self {
        Self {
            communication_token: value.communication_token,
            names: value
                .results
                .into_iter()
                .map(|result| result.name)
                .collect(),
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::CreateResultsMetaDataRequest);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: HashMap<String, ResultMetaData>,
}

impl From<Response> for v3::agent::CreateResultsMetaDataResponse {
    fn from(value: Response) -> Self {
        Self {
            communication_token: value.communication_token,
            results: value
                .results
                .into_iter()
                .map(|(k, v)| {
                    debug_assert_eq!(k, v.name);
                    v.into()
                })
                .collect(),
        }
    }
}

impl From<v3::agent::CreateResultsMetaDataResponse> for Response {
    fn from(value: v3::agent::CreateResultsMetaDataResponse) -> Self {
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

super::super::impl_convert!(req Response : v3::agent::CreateResultsMetaDataResponse);
