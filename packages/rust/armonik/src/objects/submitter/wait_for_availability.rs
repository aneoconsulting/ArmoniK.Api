use super::super::TaskError;

/// Request for waiting for a result, standing for the `ResultRequest`
/// message the stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub session_id: String,
    pub result_id: String,
}

impl From<Request> for crate::ResultRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

impl From<crate::ResultRequest> for Request {
    fn from(value: crate::ResultRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.AvailabilityReply",
    oneof = "type"
)]
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
