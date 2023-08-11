use crate::api::v3;

use super::{TaskId, TaskStatus};

#[derive(Debug, Clone, Default)]
pub struct TaskIdWithStatus {
    pub id: TaskId,
    pub status: TaskStatus,
}

impl From<TaskIdWithStatus> for v3::TaskIdWithStatus {
    fn from(value: TaskIdWithStatus) -> Self {
        Self {
            task_id: Some(value.id.into()),
            status: v3::task_status::TaskStatus::from(value.status) as i32,
        }
    }
}

impl From<v3::TaskIdWithStatus> for TaskIdWithStatus {
    fn from(value: v3::TaskIdWithStatus) -> Self {
        Self {
            id: value.task_id.unwrap_or_default().into(),
            status: value.status.into(),
        }
    }
}
