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

impl<T: Into<String>> From<T> for RequestItem {
    fn from(value: T) -> Self {
        Self {
            name: value.into(),
            manual_deletion: false,
        }
    }
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
    pub results: Vec<Raw>,
}

super::super::impl_convert!(
    struct Response = v3::results::CreateResultsMetaDataResponse {
        list results,
    }
);
