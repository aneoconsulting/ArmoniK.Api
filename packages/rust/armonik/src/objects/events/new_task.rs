use super::super::TaskStatus;

use crate::api::v3;

/// Represents an update to the status of a task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<NewTask> for v3::events::event_subscription_response::NewTask {
    fn from(value: NewTask) -> Self {
        Self {
            task_id: value.task_id,
            payload_id: value.payload_id,
            origin_task_id: value.origin_task_id,
            status: value.status as i32,
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            retry_of_ids: value.retry_of_ids,
            parent_task_ids: value.parent_task_ids,
        }
    }
}

impl From<v3::events::event_subscription_response::NewTask> for NewTask {
    fn from(value: v3::events::event_subscription_response::NewTask) -> Self {
        Self {
            task_id: value.task_id,
            payload_id: value.payload_id,
            origin_task_id: value.origin_task_id,
            status: value.status.into(),
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            retry_of_ids: value.retry_of_ids,
            parent_task_ids: value.parent_task_ids,
        }
    }
}

super::super::impl_convert!(NewTask : Option<v3::events::event_subscription_response::NewTask>);
