use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    pub session: String,
    pub task: String,
}

impl From<TaskId> for v3::TaskId {
    fn from(value: TaskId) -> Self {
        Self {
            session: value.session,
            task: value.task,
        }
    }
}

impl From<v3::TaskId> for TaskId {
    fn from(value: v3::TaskId) -> Self {
        Self {
            session: value.session,
            task: value.task,
        }
    }
}
