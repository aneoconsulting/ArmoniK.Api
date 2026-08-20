use std::collections::HashMap;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsRequest")]
pub struct Request {
    #[armonik(rename = "task_id")]
    pub task_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
pub struct Response {
    #[armonik(inlined)]
    pub task_results: HashMap<String, Vec<String>>,
}
