use super::{filter, Raw, Sort};

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.applications.ListApplicationsResponse")]
pub struct Response {
    pub applications: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
