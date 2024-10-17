use crate::api::v3;

use super::{TaskId, TaskStatus};

#[derive(Debug, Clone, Default)]
pub struct TaskIdWithStatus {
    pub id: TaskId,
    pub status: TaskStatus,
}

super::impl_convert!(
    struct TaskIdWithStatus = v3::TaskIdWithStatus {
        id = option task_id,
        status = enum status,
    }
);
