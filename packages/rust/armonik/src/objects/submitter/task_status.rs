use std::collections::HashMap;

use super::super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusRequest")]
pub struct Request {
    pub task_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusReply")]
pub struct Response {
    /// The status of each task.
    #[armonik(rename = "id_statuses")]
    pub statuses: HashMap<String, TaskStatus>,
}
