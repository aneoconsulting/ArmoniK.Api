use crate::api::v3;

use super::Raw;

/// Request for getting a single session.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session ID.
    pub id: String,
}

impl From<Request> for v3::sessions::CancelSessionRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.id,
        }
    }
}

impl From<v3::sessions::CancelSessionRequest> for Request {
    fn from(value: v3::sessions::CancelSessionRequest) -> Self {
        Self {
            id: value.session_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::sessions::CancelSessionRequest>);

/// Response for getting a single session.
///
/// Return a raw session.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The session.
    pub session: Raw,
}

impl From<Response> for v3::sessions::CancelSessionResponse {
    fn from(value: Response) -> Self {
        Self {
            session: value.session.into(),
        }
    }
}

impl From<v3::sessions::CancelSessionResponse> for Response {
    fn from(value: v3::sessions::CancelSessionResponse) -> Self {
        Self {
            session: value.session.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::sessions::CancelSessionResponse>);
