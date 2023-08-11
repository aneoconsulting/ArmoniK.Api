use crate::api::v3;

use super::PartitionRaw;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionListResponse {
    pub partitions: Vec<PartitionRaw>,
    pub page: i32,
    pub page_size: i32,
    pub total: i32,
}

impl Default for PartitionListResponse {
    fn default() -> Self {
        Self {
            partitions: Vec::new(),
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
}

impl From<PartitionListResponse> for v3::partitions::ListPartitionsResponse {
    fn from(value: PartitionListResponse) -> Self {
        Self {
            partitions: value.partitions.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

impl From<v3::partitions::ListPartitionsResponse> for PartitionListResponse {
    fn from(value: v3::partitions::ListPartitionsResponse) -> Self {
        Self {
            partitions: value.partitions.into_iter().map(Into::into).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

super::super::impl_convert!(PartitionListResponse : Option<v3::partitions::ListPartitionsResponse>);
