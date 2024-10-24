use crate::api::v3;

use super::Raw;

/// Request for pausing a single session.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session ID.
    pub id: String,
}

super::super::impl_convert!(
    struct Request = v3::sessions::PauseSessionRequest {
        id = session_id,
    }
);

/// Response for pausing a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The session.
    pub session: Raw,
}

super::super::impl_convert!(
    struct Response = v3::sessions::PauseSessionResponse {
        session = option session,
    }
);
