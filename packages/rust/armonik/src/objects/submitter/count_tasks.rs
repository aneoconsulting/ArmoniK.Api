use std::collections::HashMap;

use super::super::TaskStatus;

/// Request for counting tasks; stands in for the `TaskFilter` message at the
/// Submitter.CountTasks RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct Request {
    pub filter: super::TaskFilter,
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
