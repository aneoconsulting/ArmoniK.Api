use super::Raw;

/// Request for getting a single session.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.GetSessionRequest")]
pub struct Request {
    /// The session ID.
    pub session_id: String,
}

/// Response for getting a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.GetSessionResponse")]
pub struct Response {
    /// The session.
    pub session: Raw,
}
