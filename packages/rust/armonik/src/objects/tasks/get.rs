use super::Raw;

/// Request for getting a single task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetTaskRequest")]
pub struct Request {
    /// The task ID.
    pub task_id: String,
}

/// Response for getting a single task.
///
/// Return a detailed task.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetTaskResponse")]
pub struct Response {
    /// The task.
    pub task: Raw,
}
