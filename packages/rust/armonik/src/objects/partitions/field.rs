/// Represents every available field in a partition.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.partitions.PartitionField")]
pub enum Field {
    /// The partition ID.
    #[default]
    Id,
    /// The parent partition IDs.
    ParentPartitionIds,
    /// Whether the partition is reserved for pods.
    PodReserved,
    /// The maximum number of pods that can be used by sessions using the partition.
    PodMax,
    /// The percentage of the partition that can be preempted.
    PreemptionPercentage,
    /// The priority of the partition.
    Priority,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherField),
}
