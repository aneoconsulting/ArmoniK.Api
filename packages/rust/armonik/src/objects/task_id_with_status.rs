use crate::api::v3;

use super::{TaskId, TaskStatus};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskIdWithStatus {
    pub task_id: TaskId,
    pub status: TaskStatus,
}

super::impl_convert!(
    struct TaskIdWithStatus = v3::TaskIdWithStatus {
        task_id = option task_id,
        status = enum status,
    }
);
