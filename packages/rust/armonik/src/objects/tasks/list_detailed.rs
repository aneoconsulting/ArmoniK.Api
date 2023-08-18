use crate::api::v3;

use super::{filter, Raw, Sort};

/// Request to list tasks.
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

impl From<Request> for v3::tasks::ListTasksRequest {
    fn from(value: Request) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: Some(v3::tasks::list_tasks_request::Sort {
                field: value.sort.field.into(),
                direction: value.sort.direction as i32,
            }),
            with_errors: value.with_errors,
        }
    }
}

impl From<v3::tasks::ListTasksRequest> for Request {
    fn from(value: v3::tasks::ListTasksRequest) -> Self {
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
            with_errors: value.with_errors,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::tasks::ListTasksRequest>);

/// Response to list tasks.
///
/// Use pagination, filtering and sorting from the request.
/// Return a list of detailed tasks.
#[derive(Debug, Clone)]
pub struct Response {
    /// The list of detailed tasks.
    pub tasks: Vec<Raw>,
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

impl From<Response> for v3::tasks::ListTasksDetailedResponse {
    fn from(value: Response) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::tasks::ListTasksDetailedResponse> for Response {
    fn from(value: v3::tasks::ListTasksDetailedResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::tasks::ListTasksDetailedResponse>);
