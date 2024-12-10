use crate::api::v3;

use super::Raw;

/// Request for getting a single session.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session ID.
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::sessions::GetSessionRequest {
        session_id,
    }
);

/// Response for getting a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The session.
    pub session: Raw,
}

super::super::impl_convert!(
    struct Response = v3::sessions::GetSessionResponse {
        session = option session,
    }
);
