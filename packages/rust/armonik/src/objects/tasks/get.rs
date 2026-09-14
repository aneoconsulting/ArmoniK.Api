use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.tasks.GetTaskRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub task_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.tasks.GetTaskResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub task: Raw,
}
