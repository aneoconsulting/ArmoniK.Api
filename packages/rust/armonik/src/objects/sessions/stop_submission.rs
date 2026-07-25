use super::Raw;

/// Request for stopping new tasks submissions from clients or workers in the given session.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.StopSubmissionRequest")]
pub struct Request {
    /// The session ID.
    pub session_id: String,
    /// Whether to stop client submission.
    pub client: bool,
    /// Whether to stop worker submission.
    pub worker: bool,
}

/// Response for stopping new tasks submissions from clients or workers in the given session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.StopSubmissionResponse")]
pub struct Response {
    /// The session.
    pub session: Raw,
}
