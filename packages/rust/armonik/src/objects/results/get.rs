use super::Raw;

use crate::api::v3;

/// Request to get an result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Result id. Must fail when name is empty.
    pub id: String,
}

impl From<Request> for v3::results::GetResultRequest {
    fn from(value: Request) -> Self {
        Self {
            result_id: value.id,
        }
    }
}

impl From<v3::results::GetResultRequest> for Request {
    fn from(value: v3::results::GetResultRequest) -> Self {
        Self {
            id: value.result_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::results::GetResultRequest>);

/// Response to get an result.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The result.
    pub result: Raw,
}

impl From<Response> for v3::results::GetResultResponse {
    fn from(value: Response) -> Self {
        Self {
            result: value.result.into(),
        }
    }
}

impl From<v3::results::GetResultResponse> for Response {
    fn from(value: v3::results::GetResultResponse) -> Self {
        Self {
            result: value.result.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::results::GetResultResponse>);
