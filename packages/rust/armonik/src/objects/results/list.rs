use crate::api::v3;

use super::{filter, Raw, Sort};

/// Request to list results.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl Default for Request {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: Default::default(),
            sort: Default::default(),
        }
    }
}

super::super::impl_convert!(
    struct Request = v3::results::ListResultsRequest {
        page,
        page_size,
        filters = option filters,
        sort = option sort,
    }
);

/// Response to list results.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of raw results.
#[derive(Debug, Clone)]
pub struct Response {
    /// The list of raw results.
    pub results: Vec<Raw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of results.
    pub total: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

super::super::impl_convert!(
    struct Response = v3::results::ListResultsResponse {
        list results,
        page,
        page_size,
        total,
    }
);
