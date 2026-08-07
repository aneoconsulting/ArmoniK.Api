use super::{filter, Raw, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub page: i32,
    pub page_size: i32,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsResponse")]
pub struct Response {
    pub results: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
