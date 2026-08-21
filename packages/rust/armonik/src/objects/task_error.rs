use super::Error;

#[armonik_macros::message("armonik.api.grpc.v1.TaskError")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskError {
    pub task_id: String,
    pub errors: Vec<Error>,
}
