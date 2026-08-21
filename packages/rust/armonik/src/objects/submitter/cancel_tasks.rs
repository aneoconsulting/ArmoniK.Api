/// Request for cancelling tasks; stands in for the `TaskFilter` message at
/// the Submitter.CancelTasks RPC.
#[armonik_macros::message("armonik.api.grpc.v1.submitter.TaskFilter")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
pub struct Request {
    pub filter: super::TaskFilter,
}

#[armonik_macros::message("armonik.api.grpc.v1.Empty")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {}
