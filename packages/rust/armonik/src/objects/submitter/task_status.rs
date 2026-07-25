use std::collections::HashMap;

use super::super::TaskStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusRequest")]
pub struct Request {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusReply")]
pub struct Response {
    /// The status of each task.
    #[armonik(rename = "id_statuses", with = "crate::codec::adapters::PairMap<1, 2>")]
    pub statuses: HashMap<String, TaskStatus>,
}
