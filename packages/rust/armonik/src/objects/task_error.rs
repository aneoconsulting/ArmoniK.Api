use super::Error;

use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskError {
    pub task_id: String,
    pub errors: Vec<Error>,
}

super::impl_convert!(
    struct TaskError = v3::TaskError {
        task_id,
        list errors,
    }
);
