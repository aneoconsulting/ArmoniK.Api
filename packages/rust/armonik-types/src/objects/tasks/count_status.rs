use super::{super::StatusCount, filter};

/// Request to get count from tasks by status.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.CountTasksByStatusRequest")]
pub struct Request {
    /// The filters.
    pub filters: filter::Or,
}

/// Response to get count from tasks by status.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.CountTasksByStatusResponse")]
pub struct Response {
    /// Number of tasks by status. Expected to have only 1 object by tasks status.
    pub status: Vec<StatusCount>,
}
