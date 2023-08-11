use super::Error;

use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct TaskError {
    pub task_id: String,
    pub errors: Vec<Error>,
}

impl From<TaskError> for v3::TaskError {
    fn from(value: TaskError) -> Self {
        Self {
            task_id: value.task_id,
            errors: value.errors.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::TaskError> for TaskError {
    fn from(value: v3::TaskError) -> Self {
        Self {
            task_id: value.task_id,
            errors: value.errors.into_iter().map(Into::into).collect(),
        }
    }
}

super::impl_convert!(TaskError : Option<v3::TaskError>);
