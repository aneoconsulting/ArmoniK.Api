use super::Error;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.TaskError")]
pub struct TaskError {
    pub task_id: String,
    pub errors: Vec<Error>,
}
