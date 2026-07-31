/// Request for counting tasks; stands in for the `TaskFilter` message at the
/// Submitter.CountTasks RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.CountTasksRequest",
    service = "Submitter",
    method = "CountTasks",
    input,
))]
pub struct Request {
    pub filter: super::TaskFilter,
}

pub type Response = super::super::Count;
