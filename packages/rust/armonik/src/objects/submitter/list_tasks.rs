/// Request for listing tasks; stands in for the `TaskFilter` message at the
/// Submitter.ListTasks RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct Request {
    pub filter: super::TaskFilter,
}

/// Response for listing tasks; stands in for the `TaskIdList` message at the
/// Submitter.ListTasks RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.TaskIdList")]
pub struct Response {
    pub task_ids: Vec<String>,
}
