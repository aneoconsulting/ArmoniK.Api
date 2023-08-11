use std::collections::HashMap;

use crate::api::v3;

/// A raw partition object.
///
/// Used when a list or a single partition is returned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartitionRaw {
    /// The partition ID.
    pub id: String,
    /// The parent partition IDs.
    pub parent_partition_ids: Vec<String>,
    /// Whether the partition is reserved for pods.
    pub pod_reserved: i64,
    /// The maximum number of pods that can be used by sessions using the partition.
    pub pod_max: i64,
    /// The pod configuration.
    pub pod_configuration: HashMap<String, String>,
    /// The percentage of the partition that can be preempted.
    pub preemption_percentage: i64,
    /// The priority of the partition.
    pub priority: i64,
}

impl From<PartitionRaw> for v3::partitions::PartitionRaw {
    fn from(value: PartitionRaw) -> Self {
        Self {
            id: value.id,
            parent_partition_ids: value.parent_partition_ids,
            pod_reserved: value.pod_reserved,
            pod_max: value.pod_max,
            pod_configuration: value.pod_configuration,
            preemption_percentage: value.preemption_percentage,
            priority: value.priority,
        }
    }
}

impl From<v3::partitions::PartitionRaw> for PartitionRaw {
    fn from(value: v3::partitions::PartitionRaw) -> Self {
        Self {
            id: value.id,
            parent_partition_ids: value.parent_partition_ids,
            pod_reserved: value.pod_reserved,
            pod_max: value.pod_max,
            pod_configuration: value.pod_configuration,
            preemption_percentage: value.preemption_percentage,
            priority: value.priority,
        }
    }
}

super::super::impl_convert!(PartitionRaw : Option<v3::partitions::PartitionRaw>);
