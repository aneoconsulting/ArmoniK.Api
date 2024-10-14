use crate::api::v3;

use super::Raw;

/// Request for stopping new tasks submissions from clients or workers in the given session.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session ID.
    pub id: String,
    /// Whether to stop client submission.
    pub client: bool,
    /// Whether to stop worker submission.
    pub worker: bool,
}

super::super::impl_convert!(
    struct Request = v3::sessions::StopSubmissionRequest {
        id = session_id,
        client,
        worker,
    }
);

/// Response for stopping new tasks submissions from clients or workers in the given session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The session.
    pub session: Raw,
}

super::super::impl_convert!(
    struct Response = v3::sessions::StopSubmissionResponse {
        session = option session,
    }
);
