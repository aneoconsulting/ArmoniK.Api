use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCount {
    pub status: TaskStatus,
    pub count: i32,
}

impl From<StatusCount> for v3::StatusCount {
    fn from(value: StatusCount) -> Self {
        Self {
            status: v3::task_status::TaskStatus::from(value.status) as i32,
            count: value.count,
        }
    }
}

impl From<v3::StatusCount> for StatusCount {
    fn from(value: v3::StatusCount) -> Self {
        Self {
            status: value.status.into(),
            count: value.count,
        }
    }
}
