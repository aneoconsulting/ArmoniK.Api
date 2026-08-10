use super::TaskId;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.TaskList")]
pub struct TaskList {
    pub task_ids: Vec<TaskId>,
}
