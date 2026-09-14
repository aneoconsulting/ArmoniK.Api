use std::collections::HashMap;

#[armonik_macros::message("armonik.api.grpc.v1.tasks.GetResultIdsRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "task_id")]
    pub task_ids: Vec<String>,
}

#[armonik_macros::message("armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub task_results: HashMap<String, Vec<String>>,
}
