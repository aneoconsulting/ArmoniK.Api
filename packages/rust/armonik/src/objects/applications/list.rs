use crate::api::v3;

use super::{filter, Raw, Sort};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

super::super::impl_convert!(struct Request = v3::applications::ListApplicationsRequest { page, page_size, filters = option filters, sort = option sort });

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

super::super::impl_convert!(struct Response = v3::applications::ListApplicationsResponse { list applications, page, page_size, total });
