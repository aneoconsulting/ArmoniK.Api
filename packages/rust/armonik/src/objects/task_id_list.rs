use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskIdList {
    pub task_ids: Vec<String>,
}

super::impl_convert!(
    struct TaskIdList = v3::TaskIdList {
        task_ids,
    }
);
