use super::{filter, Raw, Sort};

/// Request to list sessions.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsRequest")]
pub struct Request {
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The filters.
    pub filters: filter::Or,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: Sort,
    /// Flag to tell if server must return task options in summary sessions
    pub with_task_options: bool,
}

/// Response to list sessions.
///
/// Use pagination, filtering and sorting from the request.
/// Return a list of summary sessions.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.ListSessionsResponse")]
pub struct Response {
    /// The list of sessions.
    pub sessions: Vec<Raw>,
    /// The current page. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of sessions.
    pub total: i32,
}
