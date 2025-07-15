use std::collections::HashMap;

use super::Raw;

use crate::api::v3;

/// Result to create without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The session in which create results.
    pub manual_deletion: bool,
}

super::super::impl_convert!(
  struct RequestItem = v3::results::create_results_meta_data_request::ResultCreate {
      name,
      manual_deletion,
  }
);

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

super::super::impl_convert!(
  struct Request = v3::results::CreateResultsMetaDataRequest {
      list results = list results,
      session_id,
  }
);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The list of raw results that were created.
    pub results: HashMap<String, Raw>,
}

impl From<Response> for v3::results::CreateResultsMetaDataResponse {
    fn from(value: Response) -> Self {
        Self {
            results: value
                .results
                .into_iter()
                .map(|(name, result)| {
                    debug_assert_eq!(name, result.name);
                    result.into()
                })
                .collect(),
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
