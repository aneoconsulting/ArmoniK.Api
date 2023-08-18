use super::Raw;

use crate::api::v3;

/// Request to get a partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The partition ID.
    pub id: String,
}

super::super::impl_convert!(
    struct Request = v3::partitions::GetPartitionRequest {
        id,
    }
);

/// Response to get a partition.
///
/// Return a raw partition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// The raw partition.
    pub partition: Raw,
}

super::super::impl_convert!(
    struct Response = v3::partitions::GetPartitionResponse {
        partition = option partition,
    }
);
