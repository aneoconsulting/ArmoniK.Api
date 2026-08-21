use super::super::TaskStatus;

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionResponse.TaskStatusUpdate")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskStatusUpdate {
    pub task_id: String,
    pub status: TaskStatus,
}
