use crate::api::v3;

/// Request deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the results to delete.
    pub result_ids: Vec<String>,
}

impl From<Request> for v3::results::DeleteResultsDataRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_ids,
        }
    }
}

impl From<v3::results::DeleteResultsDataRequest> for Request {
    fn from(value: v3::results::DeleteResultsDataRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_ids: value.result_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::results::DeleteResultsDataRequest>);

/// Response deleting data from results results but keeping metadata.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The session of the results.
    pub session_id: String,
    /// The ID of the deleted results.
    pub result_ids: Vec<String>,
}

impl From<Response> for v3::results::DeleteResultsDataResponse {
    fn from(value: Response) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_ids,
        }
    }
}

impl From<v3::results::DeleteResultsDataResponse> for Response {
    fn from(value: v3::results::DeleteResultsDataResponse) -> Self {
        Self {
            session_id: value.session_id,
            result_ids: value.result_id,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::results::DeleteResultsDataResponse>);
