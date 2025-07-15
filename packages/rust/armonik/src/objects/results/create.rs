use super::Raw;

use crate::api::v3;

/// Result to create with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The data associated to the result to create.
    pub data: Vec<u8>,
    /// The session in which create results.
    pub manual_deletion: bool,
}

impl<K: Into<String>, V: Into<Vec<u8>>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
            manual_deletion: false,
        }
    }
}

super::super::impl_convert!(
  struct RequestItem = v3::results::create_results_request::ResultCreate {
      name,
      data,
      manual_deletion,
  }
);

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

super::super::impl_convert!(
  struct Request = v3::results::CreateResultsRequest {
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
    struct Response = v3::results::CreateResultsResponse {
        list results,
    }
);
