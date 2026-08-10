use super::super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.TaskStatusUpdate")]
pub struct TaskStatusUpdate {
    pub task_id: String,
    pub status: TaskStatus,
}
