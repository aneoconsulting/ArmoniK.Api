use super::TaskDetailed;

use crate::api::v3;

/// Request for getting a single task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GetTaskRequest {
    /// The task ID.
    pub task_id: String,
}

impl From<GetTaskRequest> for v3::tasks::GetTaskRequest {
    fn from(value: GetTaskRequest) -> Self {
        Self {
            task_id: value.task_id,
        }
    }
}

impl From<v3::tasks::GetTaskRequest> for GetTaskRequest {
    fn from(value: v3::tasks::GetTaskRequest) -> Self {
        Self {
            task_id: value.task_id,
        }
    }
}

super::super::impl_convert!(GetTaskRequest : Option<v3::tasks::GetTaskRequest>);

/// Response for getting a single task.
///
/// Return a detailed task.
#[derive(Debug, Clone, Default)]
pub struct GetTaskResponse {
    /// The task.
    pub task: TaskDetailed,
}

impl From<GetTaskResponse> for v3::tasks::GetTaskResponse {
    fn from(value: GetTaskResponse) -> Self {
        Self {
            task: value.task.into(),
        }
    }
}

impl From<v3::tasks::GetTaskResponse> for GetTaskResponse {
    fn from(value: v3::tasks::GetTaskResponse) -> Self {
        Self {
            task: value.task.into(),
        }
    }
}

super::super::impl_convert!(GetTaskResponse : Option<v3::tasks::GetTaskResponse>);
