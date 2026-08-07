use super::{filter, Sort, Summary};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub with_errors: bool,
    pub page: i32,
    pub page_size: i32,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksResponse")]
pub struct Response {
    pub tasks: Vec<Summary>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
