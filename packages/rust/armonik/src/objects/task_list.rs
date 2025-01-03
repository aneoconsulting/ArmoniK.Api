use crate::api::v3;

use super::TaskId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskList {
    pub task_ids: Vec<TaskId>,
}

super::impl_convert!(
    struct TaskList = v3::TaskList {
        list task_ids,
    }
);
