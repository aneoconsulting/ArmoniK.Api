use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskIdList {
    pub ids: Vec<String>,
}

super::impl_convert!(
    struct TaskIdList = v3::TaskIdList {
        ids = task_ids,
    }
);
