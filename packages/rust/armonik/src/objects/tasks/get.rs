use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetTaskRequest")]
pub struct Request {
    pub task_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetTaskResponse")]
pub struct Response {
    pub task: Raw,
}
