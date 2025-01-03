use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskId {
    pub session_id: String,
    pub task_id: String,
}

super::impl_convert!(
    struct TaskId = v3::TaskId {
        session_id = session,
        task_id = task,
    }
);
