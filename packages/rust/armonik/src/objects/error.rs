use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default)]
pub struct Error {
    pub task_status: TaskStatus,
    pub details: String,
}

impl From<Error> for v3::Error {
    fn from(value: Error) -> Self {
        Self {
            task_status: value.task_status as i32,
            detail: value.details,
        }
    }
}

impl From<v3::Error> for Error {
    fn from(value: v3::Error) -> Self {
        Self {
            task_status: value.task_status.into(),
            details: value.detail,
        }
    }
}
super::impl_convert!(Error : Option<v3::Error>);
