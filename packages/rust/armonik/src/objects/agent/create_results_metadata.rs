use super::ResultMetaData;

use crate::api::v3;

/// Result to create without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
}

impl<T: Into<String>> From<T> for RequestItem {
    fn from(value: T) -> Self {
        Self { name: value.into() }
    }
}

super::super::impl_convert!(
  struct RequestItem = v3::agent::create_results_meta_data_request::ResultCreate {
      name,
  }
);

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of names for the results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::agent::CreateResultsMetaDataRequest {
        communication_token,
        list results,
        session_id,
    }
);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The list of ResultMetaData results that were created.
    pub results: Vec<ResultMetaData>,
}

super::super::impl_convert!(
    struct Response = v3::agent::CreateResultsMetaDataResponse {
        communication_token,
        list results,
    }
);
