use super::ResultMetaData;

use crate::api::v3;

/// Result to create with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestItem {
    /// The name of the result to create.
    pub name: String,
    /// The data associated to the result to create.
    pub data: Vec<u8>,
}

impl<K: Into<String>, V: Into<Vec<u8>>> From<(K, V)> for RequestItem {
    fn from((name, data): (K, V)) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
        }
    }
}

super::super::impl_convert!(
  struct RequestItem = v3::agent::create_results_request::ResultCreate {
      name,
      data,
  }
);

/// Request for creating results with data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Results to create.
    pub results: Vec<RequestItem>,
    /// The session in which create results.
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::agent::CreateResultsRequest {
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
    struct Response = v3::agent::CreateResultsResponse{
        communication_token,
        list results,
    }
);
