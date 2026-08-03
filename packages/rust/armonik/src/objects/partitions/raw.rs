use std::collections::HashMap;

/// A raw partition object.
///
/// Used when a list or a single partition is returned.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.PartitionRaw")]
pub struct Raw {
    /// The partition ID.
    #[armonik(rename = "id")]
    pub partition_id: String,
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
