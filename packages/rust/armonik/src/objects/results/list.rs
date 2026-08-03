use super::{filter, Raw, Sort};

/// Request to list results.
///
/// Use pagination, filtering and sorting.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsRequest")]
pub struct Request {
    /// The filters.
    pub filters: filter::Or,
    /// The sort.
    ///
    /// Must be set for every request.
    pub sort: Sort,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
}

/// Response to list results.
///
/// Use pagination, filtering and sorting from the request.
/// Return a list of raw results.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.results.ListResultsResponse")]
pub struct Response {
    /// The list of raw results.
    pub results: Vec<Raw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of results.
    pub total: i32,
}
