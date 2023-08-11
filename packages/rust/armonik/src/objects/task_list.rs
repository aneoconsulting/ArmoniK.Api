use crate::api::v3;

use super::TaskId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskList {
    pub ids: Vec<TaskId>,
}

impl From<TaskList> for v3::TaskList {
    fn from(value: TaskList) -> Self {
        Self {
            task_ids: value.ids.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::TaskList> for TaskList {
    fn from(value: v3::TaskList) -> Self {
        Self {
            ids: value.task_ids.into_iter().map(Into::into).collect(),
        }
    }
}
