use super::TaskId;

#[armonik_macros::message("armonik.api.grpc.v1.TaskList")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskList {
    pub task_ids: Vec<TaskId>,
}
