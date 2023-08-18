use super::Summary;

use crate::api::v3;

/// Request to cancel one or many tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Ids of the tasks to cancel.
    pub task_ids: Vec<String>,
}

impl From<Request> for v3::tasks::CancelTasksRequest {
    fn from(value: Request) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}

impl From<v3::tasks::CancelTasksRequest> for Request {
    fn from(value: v3::tasks::CancelTasksRequest) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::tasks::CancelTasksRequest>);

/// Response from canceling one or many tasks.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// Tasks that have been asked to cancel.
    pub tasks: Vec<Summary>,
}

impl From<Response> for v3::tasks::CancelTasksResponse {
    fn from(value: Response) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::CancelTasksResponse> for Response {
    fn from(value: v3::tasks::CancelTasksResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::tasks::CancelTasksResponse>);
