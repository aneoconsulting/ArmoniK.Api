use super::Raw;

/// Request for purging a single session.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.PurgeSessionRequest")]
pub struct Request {
    /// The session ID.
    pub session_id: String,
}

/// Response for purging a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.PurgeSessionResponse")]
pub struct Response {
    /// The session.
    pub session: Raw,
}
