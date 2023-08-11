use crate::api::v3;

use super::{super::Sort, PartitionField, PartitionFilters, PartitionFiltersAnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionListRequest {
    pub page: i32,
    pub page_size: i32,
    pub filters: PartitionFilters,
    pub sort: Sort<PartitionField>,
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
                Some(sort) => Sort {
                    field: sort.field.into(),
                    direction: sort.direction.into(),
                },
                None => Default::default(),
            },
        }
    }
}

super::super::impl_convert!(PartitionListRequest : Option<v3::partitions::ListPartitionsRequest>);
