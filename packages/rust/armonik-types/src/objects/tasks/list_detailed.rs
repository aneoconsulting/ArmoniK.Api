use super::{filter, Raw, Sort};

/// Request to list tasks.
///
/// Use pagination, filtering and sorting.
///
/// Shares its wire form with [`super::list::Request`], which the client
/// and server stubs use for both RPCs; this type only exists so that the
/// two calls stay distinguishable.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl From<Request> for super::list::Request {
    fn from(value: Request) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters,
            sort: value.sort,
            with_errors: value.with_errors,
        }
    }
}

impl From<super::list::Request> for Request {
    fn from(value: super::list::Request) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters,
            sort: value.sort,
            with_errors: value.with_errors,
        }
    }
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
