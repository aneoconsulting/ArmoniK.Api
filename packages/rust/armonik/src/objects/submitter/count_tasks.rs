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
/// A scoped response rather than an alias to [`Count`](crate::Count): the convenience emitter finds
/// a type's field reflection by mangling the path written on the rpc line, and an alias has no
/// reflection of its own. Duplicating the object per RPC site is what the crate does everywhere
/// else a proto message is shared, and it is what let the second proc macro go.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.Count")]
pub struct Response {
    #[armonik(with = "crate::codec::adapters::PairMap")]
    pub values: HashMap<TaskStatus, i32>,
}
