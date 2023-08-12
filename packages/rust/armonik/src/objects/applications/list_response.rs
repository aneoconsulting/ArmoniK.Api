use crate::api::v3;

use super::ApplicationRaw;

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
