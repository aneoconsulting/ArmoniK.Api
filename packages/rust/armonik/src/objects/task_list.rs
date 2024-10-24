use crate::api::v3;

use super::TaskId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskList {
    pub ids: Vec<TaskId>,
}

super::impl_convert!(
    struct TaskList = v3::TaskList {
        list ids = list task_ids,
    }
);
