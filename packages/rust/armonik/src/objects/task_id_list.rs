#[armonik_macros::message("armonik.api.grpc.v1.TaskIdList")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskIdList {
    pub task_ids: Vec<String>,
}
