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

impl From<Request> for v3::results::ListResultsRequest {
    fn from(value: Request) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: Some(v3::results::list_results_request::Sort {
                field: value.sort.field.into(),
                direction: value.sort.direction as i32,
            }),
        }
    }
}

impl From<v3::results::ListResultsRequest> for Request {
    fn from(value: v3::results::ListResultsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => Sort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(Request : Option<v3::results::ListResultsRequest>);

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

impl From<Response> for v3::results::ListResultsResponse {
    fn from(value: Response) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::results::ListResultsResponse> for Response {
    fn from(value: v3::results::ListResultsResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::results::ListResultsResponse>);
