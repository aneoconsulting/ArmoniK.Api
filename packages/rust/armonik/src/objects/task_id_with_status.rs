use super::{TaskId, TaskStatus};

#[armonik_macros::message("armonik.api.grpc.v1.TaskIdWithStatus")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskIdWithStatus {
    pub task_id: TaskId,
    pub status: TaskStatus,
}
