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

super::super::impl_convert!(
    struct TaskStatusUpdate = v3::events::event_subscription_response::TaskStatusUpdate {
        task_id,
        status = enum status,
    }
);
