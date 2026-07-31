/// Request for cancelling tasks; stands in for the `TaskFilter` message at
/// the Submitter.CancelTasks RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.CancelTasksRequest",
    service = "Submitter",
    method = "CancelTasks",
    input,
))]
pub struct Request {
    pub filter: super::TaskFilter,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.CancelTasksResponse",
    service = "Submitter",
    method = "CancelTasks",
    output,
))]
pub struct Response {}
