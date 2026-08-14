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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.AvailabilityReply")]
pub enum Response {
    /// No member set. Distinct from [`Ok`](Self::Ok), which carries nothing but *is* set.
    #[default]
    Invalid,
    #[armonik(present)]
    Ok,
    #[armonik(rename = "error")]
    TaskError(TaskError),
    #[armonik(rename = "not_completed_task")]
    NotCompleted(String),
}
