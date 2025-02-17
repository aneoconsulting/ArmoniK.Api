use crate::api::v3;

/// Request deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the results to delete.
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::results::DeleteResultsDataRequest {
        session_id,
        result_ids = result_id,
    }
);

/// Response deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the deleted results.
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Response = v3::results::DeleteResultsDataResponse {
        session_id,
        result_ids = result_id,
    }
);
