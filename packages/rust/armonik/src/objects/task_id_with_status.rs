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
            task_id: value.id.into(),
            status: value.status as i32,
        }
    }
}

impl From<v3::TaskIdWithStatus> for TaskIdWithStatus {
    fn from(value: v3::TaskIdWithStatus) -> Self {
        Self {
            id: value.task_id.into(),
            status: value.status.into(),
        }
    }
}

super::impl_convert!(TaskIdWithStatus : Option<v3::TaskIdWithStatus>);
