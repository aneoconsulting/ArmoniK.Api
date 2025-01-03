use crate::api::v3;

use super::{filter, Raw, Sort};

/// Request to list sessions.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The filters.
    pub filters: filter::Or,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: Sort,
    /// Flag to tell if server must return task options in summary sessions
    pub with_task_options: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: Default::default(),
            sort: Default::default(),
            with_task_options: false,
        }
    }
}

super::super::impl_convert!(
    struct Request = v3::sessions::ListSessionsRequest {
        page,
        page_size,
        filters = option filters,
        sort = option sort,
        with_task_options,
    }
);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub sessions: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

super::super::impl_convert!(
    struct Response = v3::sessions::ListSessionsResponse {
        list sessions,
        page,
        page_size,
        total,
    }
);
