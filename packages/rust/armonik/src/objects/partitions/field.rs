#[armonik_macros::enumeration("armonik.api.grpc.v1.partitions.PartitionField")]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent)]
pub enum Field {
    Id,
    ParentPartitionIds,
    PodReserved,
    PodMax,
    PreemptionPercentage,
    Priority,
    /// Unspecified (zero) or a field unknown to this crate version.
    Unknown(UnknownField),
}
