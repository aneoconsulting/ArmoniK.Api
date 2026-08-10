use super::super::TaskError;

/// Request for waiting for a result; stands in for the `ResultRequest`
/// message at the Submitter.WaitForAvailability RPC.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.ResultRequest")]
pub struct Request {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.AvailabilityReply")]
pub enum Response {
    #[armonik(present)]
    Ok,
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
