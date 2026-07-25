use super::Summary;

/// Request to cancel one or many tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.CancelTasksRequest")]
pub struct Request {
    /// Ids of the tasks to cancel.
    pub task_ids: Vec<String>,
}

/// Response from canceling one or many tasks.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.CancelTasksResponse")]
pub struct Response {
    /// Tasks that have been asked to cancel.
    pub tasks: Vec<Summary>,
}
