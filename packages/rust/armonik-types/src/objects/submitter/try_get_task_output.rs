/// Request for retrieving a task output; stands in for the
/// `TaskOutputRequest` message at the Submitter.TryGetTaskOutput RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskOutputRequest")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.TryGetTaskOutputRequest",
    service = "Submitter",
    method = "TryGetTaskOutput",
    input,
))]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}

pub type Response = super::super::Output;
