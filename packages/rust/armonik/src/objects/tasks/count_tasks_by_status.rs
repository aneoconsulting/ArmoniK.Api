use super::{super::StatusCount, TaskFilters};

use crate::api::v3;

/// Request to get count from tasks by status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CountTasksByStatusRequest {
    /// The filters.
    pub filters: TaskFilters,
}

impl From<CountTasksByStatusRequest> for v3::tasks::CountTasksByStatusRequest {
    fn from(value: CountTasksByStatusRequest) -> Self {
        Self {
            filters: value.filters.into(),
        }
    }
}

impl From<v3::tasks::CountTasksByStatusRequest> for CountTasksByStatusRequest {
    fn from(value: v3::tasks::CountTasksByStatusRequest) -> Self {
        Self {
            filters: value.filters.into(),
        }
    }
}

super::super::impl_convert!(CountTasksByStatusRequest : Option<v3::tasks::CountTasksByStatusRequest>);

/// Response to get count from tasks by status.
#[derive(Debug, Clone, Default)]
pub struct CountTasksByStatusResponse {
    /// Number of tasks by status. Expected to have only 1 objct by tasks status.
    pub status: Vec<StatusCount>,
}

impl From<CountTasksByStatusResponse> for v3::tasks::CountTasksByStatusResponse {
    fn from(value: CountTasksByStatusResponse) -> Self {
        Self {
            status: value.status.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::CountTasksByStatusResponse> for CountTasksByStatusResponse {
    fn from(value: v3::tasks::CountTasksByStatusResponse) -> Self {
        Self {
            status: value.status.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(CountTasksByStatusResponse : Option<v3::tasks::CountTasksByStatusResponse>);
