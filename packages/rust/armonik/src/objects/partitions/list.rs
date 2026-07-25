use super::{filter, Raw, Sort};

/// Request to list partitions.
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.ListPartitionsRequest")]
pub struct Request {
    /// The page number. Start at 0.
    pub page: i32,
    /// The number of items per page.
    pub page_size: i32,
    /// The filter.
    pub filters: filter::Or,
    /// The sort.
    ///
    /// Must be set for every request.
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

/// Response to list partitions.
///
/// Use pagination, filtering and sorting from the request.
/// Retunr a list of raw partitions.
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.ListPartitionsResponse")]
pub struct Response {
    /// The list of raw partitions.
    pub partitions: Vec<Raw>,
    /// The page number. Start at 0.
    pub page: i32,
    /// The page size.
    pub page_size: i32,
    /// The total number of partitions.
    pub total: i32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            partitions: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}
