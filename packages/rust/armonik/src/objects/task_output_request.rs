use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskOutputRequest {
    pub session: String,
    pub task_id: String,
}

impl From<TaskOutputRequest> for v3::TaskOutputRequest {
    fn from(value: TaskOutputRequest) -> Self {
        Self {
            session: value.session,
            task_id: value.task_id,
        }
    }
}

impl From<v3::TaskOutputRequest> for TaskOutputRequest {
    fn from(value: v3::TaskOutputRequest) -> Self {
        Self {
            session: value.session,
            task_id: value.task_id,
        }
    }
}
