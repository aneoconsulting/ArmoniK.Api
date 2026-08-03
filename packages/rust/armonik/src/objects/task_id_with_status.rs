use super::{TaskId, TaskStatus};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskIdWithStatus")]
pub struct TaskIdWithStatus {
    pub task_id: TaskId,
    pub status: TaskStatus,
}
