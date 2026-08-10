#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.TaskIdList")]
pub struct TaskIdList {
    pub task_ids: Vec<String>,
}
