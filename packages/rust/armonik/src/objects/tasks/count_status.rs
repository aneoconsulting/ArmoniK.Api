use super::{super::StatusCount, filter};

use crate::api::v3;

/// Request to get count from tasks by status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The filters.
    pub filters: filter::Or,
}

impl From<Request> for v3::tasks::CountTasksByStatusRequest {
    fn from(value: Request) -> Self {
        Self {
            filters: value.filters.into(),
        }
    }
}

impl From<v3::tasks::CountTasksByStatusRequest> for Request {
    fn from(value: v3::tasks::CountTasksByStatusRequest) -> Self {
        Self {
            filters: value.filters.into(),
        }
    }
}

super::super::impl_convert!(Request : Option<v3::tasks::CountTasksByStatusRequest>);

/// Response to get count from tasks by status.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// Number of tasks by status. Expected to have only 1 objct by tasks status.
    pub status: Vec<StatusCount>,
}

impl From<Response> for v3::tasks::CountTasksByStatusResponse {
    fn from(value: Response) -> Self {
        Self {
            status: value.status.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::CountTasksByStatusResponse> for Response {
    fn from(value: v3::tasks::CountTasksByStatusResponse) -> Self {
        Self {
            status: value.status.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::tasks::CountTasksByStatusResponse>);
