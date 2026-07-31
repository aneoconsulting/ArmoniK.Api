use super::{filter, Raw, Sort};

/// Request to list tasks.
///
/// Use pagination, filtering and sorting.
///
/// Shares its wire form (`ListTasksRequest`) with [`super::list::Request`];
/// the build script gives this RPC a distinct synthetic stub message so the
/// two calls stay fully distinct types.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksRequest")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.tasks.ListTasksDetailedRequest",
    service = "Tasks",
    method = "ListTasksDetailed",
    input,
))]
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
    /// Request error message in case of error in task.
    pub with_errors: bool,
}

/// Response to list tasks.
///
/// Use pagination, filtering and sorting from the request.
/// Return a list of detailed tasks.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.ListTasksDetailedResponse")]
pub struct Response {
    /// The list of detailed tasks.
    pub tasks: Vec<Raw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of tasks.
    pub total: i32,
}
