use crate::api::v3;

use super::{filter, Raw, Sort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub page: i32,
    pub page_size: i32,
    pub filters: filter::Or,
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

impl From<Request> for v3::applications::ListApplicationsRequest {
    fn from(value: Request) -> Self {
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

impl From<v3::applications::ListApplicationsRequest> for Request {
    fn from(value: v3::applications::ListApplicationsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => Sort {
                    fields: sort.fields.into_iter().map(Into::into).collect(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(Request : Option<v3::applications::ListApplicationsRequest>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub applications: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            applications: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<Response> for v3::applications::ListApplicationsResponse {
    fn from(value: Response) -> Self {
        Self {
            applications: value.applications.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::applications::ListApplicationsResponse> for Response {
    fn from(value: v3::applications::ListApplicationsResponse) -> Self {
        Self {
            applications: value.applications.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::applications::ListApplicationsResponse>);
