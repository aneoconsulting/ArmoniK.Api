use super::{filter, Raw, Sort};

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsRequest")]
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

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsResponse")]
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
