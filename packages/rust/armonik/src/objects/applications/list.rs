use crate::api::v3;

use super::{ApplicationFilters, ApplicationFiltersAnd, ApplicationRaw, ApplicationSort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationListRequest {
    pub page: i32,
    pub page_size: i32,
    pub filters: ApplicationFilters,
    pub sort: ApplicationSort,
}

impl Default for ApplicationListRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: ApplicationFilters {
                or: vec![ApplicationFiltersAnd::default()],
            },
            sort: Default::default(),
        }
    }
}

impl From<ApplicationListRequest> for v3::applications::ListApplicationsRequest {
    fn from(value: ApplicationListRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: Some(v3::applications::list_applications_request::Sort {
                fields: value.sort.fields.into_iter().map(Into::into).collect(),
                direction: value.sort.direction as i32,
            }),
        }
    }
}

impl From<v3::applications::ListApplicationsRequest> for ApplicationListRequest {
    fn from(value: v3::applications::ListApplicationsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => ApplicationSort {
                    fields: sort.fields.into_iter().map(Into::into).collect(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(ApplicationListRequest : Option<v3::applications::ListApplicationsRequest>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationListResponse {
    pub applications: Vec<ApplicationRaw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}

impl Default for ApplicationListResponse {
    fn default() -> Self {
        Self {
            applications: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<ApplicationListResponse> for v3::applications::ListApplicationsResponse {
    fn from(value: ApplicationListResponse) -> Self {
        Self {
            applications: value.applications.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::applications::ListApplicationsResponse> for ApplicationListResponse {
    fn from(value: v3::applications::ListApplicationsResponse) -> Self {
        Self {
            applications: value.applications.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(ApplicationListResponse : Option<v3::applications::ListApplicationsResponse>);
