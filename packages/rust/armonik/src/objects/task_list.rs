use super::TaskId;

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskList")]
pub struct TaskList {
    pub task_ids: Vec<TaskId>,
}
