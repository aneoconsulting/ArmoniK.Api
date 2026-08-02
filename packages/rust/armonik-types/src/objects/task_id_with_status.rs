use super::{TaskId, TaskStatus};

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskIdWithStatus")]
pub struct TaskIdWithStatus {
    pub task_id: TaskId,
    pub status: TaskStatus,
}
