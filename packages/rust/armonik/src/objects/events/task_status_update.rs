use super::super::TaskStatus;

use crate::api::v3;

/// Represents an update to the status of a task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskStatusUpdate {
    /// The task id.
    pub task_id: String,
    /// The task status.
    pub status: TaskStatus,
}

impl From<TaskStatusUpdate> for v3::events::event_subscription_response::TaskStatusUpdate {
    fn from(value: TaskStatusUpdate) -> Self {
        Self {
            task_id: value.task_id,
            status: value.status as i32,
        }
    }
}

impl From<v3::events::event_subscription_response::TaskStatusUpdate> for TaskStatusUpdate {
    fn from(value: v3::events::event_subscription_response::TaskStatusUpdate) -> Self {
        Self {
            task_id: value.task_id,
            status: value.status.into(),
        }
    }
}

super::super::impl_convert!(TaskStatusUpdate : Option<v3::events::event_subscription_response::TaskStatusUpdate>);
