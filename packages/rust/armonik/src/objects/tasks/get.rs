use super::Raw;

use crate::api::v3;

/// Request for getting a single task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The task ID.
    pub task_id: String,
}

super::super::impl_convert!(
    struct Request = v3::tasks::GetTaskRequest {
        task_id,
    }
);

/// Response for getting a single task.
///
/// Return a detailed task.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The task.
    pub task: Raw,
}

super::super::impl_convert!(
    struct Response = v3::tasks::GetTaskResponse {
        task = option task,
    }
);
