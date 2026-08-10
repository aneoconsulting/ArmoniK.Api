/// Request for counting tasks; stands in for the `TaskFilter` message at the
/// Submitter.CountTasks RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(transparent)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct Request {
    pub filter: super::TaskFilter,
}

#[armonik_macros::reflect]
pub type Response = super::super::Count;
