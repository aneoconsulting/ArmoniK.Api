use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    pub session: String,
    pub task: String,
}

super::impl_convert!(
    struct TaskId = v3::TaskId {
        session,
        task,
    }
);
