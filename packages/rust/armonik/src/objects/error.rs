use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default)]
pub struct Error {
    pub task_status: TaskStatus,
    pub details: String,
}

super::impl_convert!(
    struct Error = v3::Error {
        task_status = enum task_status,
        details = detail,
    }
);
