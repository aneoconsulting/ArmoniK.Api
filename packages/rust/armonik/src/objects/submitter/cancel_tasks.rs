/// Request for cancelling tasks; stands in for the `TaskFilter` message at
/// the Submitter.CancelTasks RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct Request {
    pub filter: super::TaskFilter,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
pub struct Response {}
