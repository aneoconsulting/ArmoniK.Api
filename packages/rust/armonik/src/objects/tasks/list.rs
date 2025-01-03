use crate::api::v3;

use super::{filter, Sort, Summary};

/// Request to list tasks.
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
    /// Request error message in case of error in task.
    pub with_errors: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: Default::default(),
            sort: Default::default(),
            with_errors: false,
        }
    }
}

super::super::impl_convert!(
    struct Request = v3::tasks::ListTasksRequest {
        page,
        page_size,
        filters = option filters,
        sort = option sort,
        with_errors,
    }
);

/// Response to list tasks.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of tasks summary.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The list of tasks summary.
    pub tasks: Vec<Summary>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of tasks.
    pub total: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

super::super::impl_convert!(
    struct Response = v3::tasks::ListTasksResponse {
        list tasks,
        page,
        page_size,
        total,
    }
);
