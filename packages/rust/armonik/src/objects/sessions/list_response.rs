use crate::api::v3;

use super::SessionRaw;

#[derive(Debug, Clone)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionRaw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}

impl Default for SessionListResponse {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<SessionListResponse> for v3::sessions::ListSessionsResponse {
    fn from(value: SessionListResponse) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::sessions::ListSessionsResponse> for SessionListResponse {
    fn from(value: v3::sessions::ListSessionsResponse) -> Self {
        Self {
            sessions: value.sessions.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(SessionListResponse : Option<v3::sessions::ListSessionsResponse>);
