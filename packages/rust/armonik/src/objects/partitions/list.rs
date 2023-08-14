use crate::api::v3;

use super::{PartitionFilters, PartitionFiltersAnd, PartitionRaw, PartitionSort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionListRequest {
    pub page: i32,
    pub page_size: i32,
    pub filters: PartitionFilters,
    pub sort: PartitionSort,
}

impl Default for PartitionListRequest {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 100,
            filters: PartitionFilters {
                or: vec![PartitionFiltersAnd::default()],
            },
            sort: Default::default(),
        }
    }
}

impl From<PartitionListRequest> for v3::partitions::ListPartitionsRequest {
    fn from(value: PartitionListRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: Some(v3::partitions::list_partitions_request::Sort {
                field: value.sort.field.into(),
                direction: value.sort.direction as i32,
            }),
        }
    }
}

impl From<v3::partitions::ListPartitionsRequest> for PartitionListRequest {
    fn from(value: v3::partitions::ListPartitionsRequest) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            filters: value.filters.into(),
            sort: match value.sort {
                Some(sort) => PartitionSort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(PartitionListRequest : Option<v3::partitions::ListPartitionsRequest>);

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
