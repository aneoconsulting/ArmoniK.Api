use crate::api::v3;

use super::{TaskFilters, TaskFiltersAnd, TaskRaw, TaskSort, TaskSummary};

/// Request to list tasks.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListRequest {
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The filters.
    pub filters: TaskFilters,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: TaskSort,
    /// Request error message in case of error in task.
    pub with_errors: bool,
}

impl Default for TaskListRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: TaskFilters {
                or: vec![TaskFiltersAnd::default()],
            },
            sort: Default::default(),
            with_errors: false,
        }
    }
}

impl From<TaskListRequest> for v3::tasks::ListTasksRequest {
    fn from(value: TaskListRequest) -> Self {
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

impl From<v3::tasks::ListTasksRequest> for TaskListRequest {
    fn from(value: v3::tasks::ListTasksRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => TaskSort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
            with_errors: value.with_errors,
        }
    }
}

super::super::impl_convert!(TaskListRequest : Option<v3::tasks::ListTasksRequest>);

/// Response to list Tasks.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of raw Tasks.
#[derive(Debug, Clone)]
pub struct TaskListResponse {
    /// The list of raw Tasks.
    pub tasks: Vec<TaskSummary>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of tasks.
    pub total: i32,
}

impl Default for TaskListResponse {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<TaskListResponse> for v3::tasks::ListTasksResponse {
    fn from(value: TaskListResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::tasks::ListTasksResponse> for TaskListResponse {
    fn from(value: v3::tasks::ListTasksResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(TaskListResponse : Option<v3::tasks::ListTasksResponse>);

/// Response to list Tasks.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of raw Tasks.
#[derive(Debug, Clone)]
pub struct TaskListRawResponse {
    /// The list of raw Tasks.
    pub tasks: Vec<TaskRaw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of tasks.
    pub total: i32,
}

impl Default for TaskListRawResponse {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<TaskListRawResponse> for v3::tasks::ListTasksRawResponse {
    fn from(value: TaskListRawResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::tasks::ListTasksRawResponse> for TaskListRawResponse {
    fn from(value: v3::tasks::ListTasksRawResponse) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(TaskListRawResponse : Option<v3::tasks::ListTasksRawResponse>);
