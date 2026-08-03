use super::super::{DataChunk, TaskError};

/// Request for retrieving a result; stands in for the `ResultRequest` message
/// at the Submitter.TryGetResultStream RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.ResultRequest")]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.ResultReply")]
pub enum Response {
    #[armonik(rename = "result")]
    DataChunk(DataChunk),
    #[armonik(rename = "error")]
    TaskError(TaskError),
    #[armonik(rename = "not_completed_task")]
    NotCompleted(String),
}

impl Default for Response {
    fn default() -> Self {
        Self::NotCompleted(Default::default())
    }
}
