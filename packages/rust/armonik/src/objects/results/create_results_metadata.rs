use super::ResultRaw;

use crate::api::v3;

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreateResultsMetadataRequest {
    /// The list of results to create.
    pub results: Vec<String>,
    /// The session in which create results.
    pub session_id: String,
}

impl From<CreateResultsMetadataRequest> for v3::results::CreateResultsMetaDataRequest {
    fn from(value: CreateResultsMetadataRequest) -> Self {
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

impl From<v3::results::CreateResultsMetaDataRequest> for CreateResultsMetadataRequest {
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

super::super::impl_convert!(CreateResultsMetadataRequest : Option<v3::results::CreateResultsMetaDataRequest>);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
pub struct CreateResultsMetadataResponse {
    /// The list of raw results that were created.
    pub results: Vec<ResultRaw>,
}

impl From<CreateResultsMetadataResponse> for v3::results::CreateResultsMetaDataResponse {
    fn from(value: CreateResultsMetadataResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::CreateResultsMetaDataResponse> for CreateResultsMetadataResponse {
    fn from(value: v3::results::CreateResultsMetaDataResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(CreateResultsMetadataResponse : Option<v3::results::CreateResultsMetaDataResponse>);
