use super::super::{DataChunk, TaskError};

/// Request for retrieving a result; stands in for the `ResultRequest` message
/// at the Submitter.TryGetResultStream RPC.
#[armonik_macros::message("armonik.api.grpc.v1.ResultRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.submitter.ResultReply")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Response {
    /// No member set, which `NotCompleted("")` is not.
    #[default]
    Invalid,
    #[armonik(rename = "result")]
    DataChunk(DataChunk),
    #[armonik(rename = "error")]
    TaskError(TaskError),
    #[armonik(rename = "not_completed_task")]
    NotCompleted(String),
}
