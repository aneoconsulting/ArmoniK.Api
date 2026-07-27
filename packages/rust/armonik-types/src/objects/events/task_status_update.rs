use super::super::TaskStatus;

/// Represents an update to the status of a task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.TaskStatusUpdate")]
pub struct TaskStatusUpdate {
    /// The task id.
    pub task_id: String,
    /// The task status.
    pub status: TaskStatus,
}
