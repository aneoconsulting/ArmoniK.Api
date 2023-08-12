use crate::api::v3;

use super::{super::Sort, SessionField, SessionFilters, SessionFiltersAnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListRequest {
    pub page: i32,
    pub page_size: i32,
    pub filters: SessionFilters,
    pub sort: Sort<SessionField>,
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
                Some(sort) => Sort {
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
