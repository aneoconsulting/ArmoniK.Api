use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskIdList {
    pub ids: Vec<String>,
}

impl From<TaskIdList> for v3::TaskIdList {
    fn from(value: TaskIdList) -> Self {
        Self {
            task_ids: value.ids,
        }
    }
}

impl From<v3::TaskIdList> for TaskIdList {
    fn from(value: v3::TaskIdList) -> Self {
        Self {
            ids: value.task_ids,
        }
    }
}

super::impl_convert!(TaskIdList : Option<v3::TaskIdList>);
