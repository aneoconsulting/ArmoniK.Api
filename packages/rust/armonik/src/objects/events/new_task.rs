use super::super::TaskStatus;

use crate::api::v3;

/// Represents an update to the status of a task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NewTask {
    /// The task id.
    pub task_id: String,
    /// The payload id.
    pub payload_id: String,
    /// The task id before retry.
    pub origin_task_id: String,
    /// The task status.
    pub status: TaskStatus,
    /// The keys of the expected outputs
    pub expected_output_keys: Vec<String>,
    /// The keys of the data dependencies.
    pub data_dependencies: Vec<String>,
    /// The list of retried tasks from the first retry to the current.
    pub retry_of_ids: Vec<String>,
    /// The parent task IDs. A tasks can be a child of another task.
    pub parent_task_ids: Vec<String>,
}

super::super::impl_convert!(
    struct NewTask = v3::events::event_subscription_response::NewTask {
        task_id,
        payload_id,
        origin_task_id,
        status = enum status,
        expected_output_keys,
        data_dependencies,
        retry_of_ids,
        parent_task_ids,
    }
);
