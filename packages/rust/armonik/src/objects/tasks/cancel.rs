use super::TaskSummary;

use crate::api::v3;

/// Request to cancel one or many tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CancelTasksRequest {
    /// Ids of the tasks to cancel.
    pub task_ids: Vec<String>,
}

impl From<CancelTasksRequest> for v3::tasks::CancelTasksRequest {
    fn from(value: CancelTasksRequest) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}

impl From<v3::tasks::CancelTasksRequest> for CancelTasksRequest {
    fn from(value: v3::tasks::CancelTasksRequest) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}

super::super::impl_convert!(CancelTasksRequest : Option<v3::tasks::CancelTasksRequest>);

/// Response from canceling one or many tasks.
#[derive(Debug, Clone, Default)]
pub struct CancelTasksResponse {
    /// Tasks that have been asked to cancel.
    pub tasks: Vec<TaskSummary>,
}

impl From<CancelTasksResponse> for v3::tasks::CancelTasksResponse {
    fn from(value: CancelTasksResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::CancelTasksResponse> for CancelTasksResponse {
    fn from(value: v3::tasks::CancelTasksResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(CancelTasksResponse : Option<v3::tasks::CancelTasksResponse>);
