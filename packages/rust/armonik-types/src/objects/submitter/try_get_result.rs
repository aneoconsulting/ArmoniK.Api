use super::super::{DataChunk, TaskError};

/// Request for retrieving a result; stands in for the `ResultRequest` message
/// at the Submitter.TryGetResultStream RPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.ResultRequest")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.TryGetResultStreamRequest",
    service = "Submitter",
    method = "TryGetResultStream",
    input,
))]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
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
