use std::collections::HashMap;

use super::super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.WaitRequest")]
pub struct Request {
    pub filter: super::TaskFilter,
    pub stop_on_first_task_error: bool,
    pub stop_on_first_task_cancellation: bool,
}

/// Number of tasks per status, from the repeated `StatusCount` pairs (duplicate statuses collapse,
/// last wins).
///
/// A scoped response rather than an alias to [`Count`](crate::Count), like every other proto
/// message this crate shares across RPC sites: response types stay injective over RPCs.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.Count")]
pub struct Response {
    pub values: HashMap<TaskStatus, i32>,
}
