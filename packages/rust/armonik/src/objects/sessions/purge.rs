use crate::api::v3;

use super::Raw;

/// Request for purging a single session.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// The session ID.
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::sessions::PurgeSessionRequest {
        session_id,
    }
);

/// Response for purging a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The session.
    pub session: Raw,
}

super::super::impl_convert!(
    struct Response = v3::sessions::PurgeSessionResponse {
        session = option session,
    }
);
