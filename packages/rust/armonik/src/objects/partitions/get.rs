use super::Raw;

use crate::api::v3;

/// Request to get a partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// The partition ID.
    pub partition_id: String,
}

super::super::impl_convert!(
    struct Request = v3::partitions::GetPartitionRequest {
        partition_id = id,
    }
);

/// Response to get a partition.
///
/// Return a raw partition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The raw partition.
    pub partition: Raw,
}

super::super::impl_convert!(
    struct Response = v3::partitions::GetPartitionResponse {
        partition = option partition,
    }
);
