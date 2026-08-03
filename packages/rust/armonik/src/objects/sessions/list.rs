use super::{filter, Raw, Sort};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsRequest")]
pub struct Request {
    pub filters: filter::Or,
    pub sort: Sort,
    pub with_task_options: bool,
    pub page: i32,
    pub page_size: i32,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsResponse")]
pub struct Response {
    pub sessions: Vec<Raw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}
