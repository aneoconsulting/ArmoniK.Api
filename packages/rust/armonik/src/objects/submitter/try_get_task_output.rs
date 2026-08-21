/// Request for retrieving a task output; stands in for the
/// `TaskOutputRequest` message at the Submitter.TryGetTaskOutput RPC.
#[armonik_macros::message("armonik.api.grpc.v1.TaskOutputRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}

pub type Response = super::super::Output;
