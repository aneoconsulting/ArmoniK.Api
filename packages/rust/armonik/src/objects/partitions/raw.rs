use std::collections::HashMap;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.PartitionRaw")]
pub struct Raw {
    #[armonik(rename = "id")]
    pub partition_id: String,
    pub parent_partition_ids: Vec<String>,
    pub pod_reserved: i64,
    pub pod_max: i64,
    pub pod_configuration: HashMap<String, String>,
    pub preemption_percentage: i64,
    pub priority: i64,
}
