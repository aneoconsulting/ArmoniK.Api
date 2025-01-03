use super::Summary;

use crate::api::v3;

/// Request to cancel one or many tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Ids of the tasks to cancel.
    pub task_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::tasks::CancelTasksRequest {
        task_ids,
    }
);

/// Response from canceling one or many tasks.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Tasks that have been asked to cancel.
    pub tasks: Vec<Summary>,
}

super::super::impl_convert!(
    struct Response = v3::tasks::CancelTasksResponse {
        list tasks,
    }
);
