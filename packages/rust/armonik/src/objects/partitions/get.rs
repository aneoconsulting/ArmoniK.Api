use super::Raw;

use crate::api::v3;

/// Request to get a partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The partition ID.
    pub id: String,
}

impl From<Request> for v3::partitions::GetPartitionRequest {
    fn from(value: Request) -> Self {
        Self { id: value.id }
    }
}

impl From<v3::partitions::GetPartitionRequest> for Request {
    fn from(value: v3::partitions::GetPartitionRequest) -> Self {
        Self { id: value.id }
    }
}

super::super::impl_convert!(Request : Option<v3::partitions::GetPartitionRequest>);

/// Response to get a partition.
///
/// Return a raw partition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// The raw partition.
    pub partition: Raw,
}

impl From<Response> for v3::partitions::GetPartitionResponse {
    fn from(value: Response) -> Self {
        Self {
            partition: value.partition.into(),
        }
    }
}

impl From<v3::partitions::GetPartitionResponse> for Response {
    fn from(value: v3::partitions::GetPartitionResponse) -> Self {
        Self {
            partition: value.partition.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::partitions::GetPartitionResponse>);
