use super::{filter, Raw, Sort};

/// Shares its wire form (`ListTasksRequest`) with [`super::list::Request`];
/// a distinct type keeps the two RPCs' requests distinct (request types are
/// injective over RPCs).
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksDetailedResponse")]
pub struct Response {
    pub tasks: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
