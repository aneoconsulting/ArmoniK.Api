use crate::api::v3;

use super::{ResultFilters, ResultFiltersAnd, ResultRaw, ResultSort};

/// Request to list results.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultListRequest {
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The filters.
    pub filters: ResultFilters,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: ResultSort,
}

impl Default for ResultListRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: ResultFilters {
                or: vec![ResultFiltersAnd::default()],
            },
            sort: Default::default(),
        }
    }
}

impl From<ResultListRequest> for v3::results::ListResultsRequest {
    fn from(value: ResultListRequest) -> Self {
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

impl From<v3::results::ListResultsRequest> for ResultListRequest {
    fn from(value: v3::results::ListResultsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => ResultSort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(ResultListRequest : Option<v3::results::ListResultsRequest>);

/// Response to list results.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of raw results.
#[derive(Debug, Clone)]
pub struct ResultListResponse {
    /// The list of raw results.
    pub results: Vec<ResultRaw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of results.
    pub total: i32,
}

impl Default for ResultListResponse {
    fn default() -> Self {
        Self {
            results: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<ResultListResponse> for v3::results::ListResultsResponse {
    fn from(value: ResultListResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::results::ListResultsResponse> for ResultListResponse {
    fn from(value: v3::results::ListResultsResponse) -> Self {
        Self {
            results: value.results.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(ResultListResponse : Option<v3::results::ListResultsResponse>);
