use std::collections::HashMap;

use super::super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusRequest")]
pub struct Request {
    pub task_ids: Vec<String>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.GetTaskStatusReply")]
pub struct Response {
    /// The status of each task.
    #[armonik(
        rename = "id_statuses",
        with = "crate::codec::adapters::PairMap<1, 2>",
        absorbs = "armonik.api.grpc.v1.submitter.GetTaskStatusReply.IdStatus"
    )]
    pub statuses: HashMap<String, TaskStatus>,
}
