use super::Summary;

#[armonik_macros::message("armonik.api.grpc.v1.tasks.CancelTasksRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub task_ids: Vec<String>,
}

#[armonik_macros::message("armonik.api.grpc.v1.tasks.CancelTasksResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub tasks: Vec<Summary>,
}
