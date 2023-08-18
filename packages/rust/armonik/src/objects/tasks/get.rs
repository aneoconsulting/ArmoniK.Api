use super::Raw;

use crate::api::v3;

/// Request for getting a single task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The task ID.
    pub task_id: String,
}

impl From<Request> for v3::tasks::GetTaskRequest {
    fn from(value: Request) -> Self {
        Self {
            task_id: value.task_id,
        }
    }
}

impl From<v3::tasks::GetTaskRequest> for Request {
    fn from(value: v3::tasks::GetTaskRequest) -> Self {
        Self {
            task_id: value.task_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::tasks::GetTaskRequest>);

/// Response for getting a single task.
///
/// Return a detailed task.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The task.
    pub task: Raw,
}

impl From<Response> for v3::tasks::GetTaskResponse {
    fn from(value: Response) -> Self {
        Self {
            task: value.task.into(),
        }
    }
}

impl From<v3::tasks::GetTaskResponse> for Response {
    fn from(value: v3::tasks::GetTaskResponse) -> Self {
        Self {
            task: value.task.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::tasks::GetTaskResponse>);
