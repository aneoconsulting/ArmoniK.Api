use crate::api::v3;

use super::{SessionFilters, SessionFiltersAnd, SessionRaw, SessionSort};

/// Request to list sessions.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListRequest {
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The filters.
    pub filters: SessionFilters,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: SessionSort,
    /// Flag to tell if server must return task options in summary sessions
    pub with_task_options: bool,
}

impl Default for SessionListRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: SessionFilters {
                or: vec![SessionFiltersAnd::default()],
            },
            sort: Default::default(),
            with_task_options: false,
        }
    }
}

impl From<SessionListRequest> for v3::sessions::ListSessionsRequest {
    fn from(value: SessionListRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: Some(v3::sessions::list_sessions_request::Sort {
                field: value.sort.field.into(),
                direction: value.sort.direction as i32,
            }),
            with_task_options: value.with_task_options,
        }
    }
}

impl From<v3::sessions::ListSessionsRequest> for SessionListRequest {
    fn from(value: v3::sessions::ListSessionsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => SessionSort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
            with_task_options: value.with_task_options,
        }
    }
}

super::super::impl_convert!(SessionListRequest : Option<v3::sessions::ListSessionsRequest>);

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
