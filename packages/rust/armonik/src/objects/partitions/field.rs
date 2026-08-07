#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.partitions.PartitionField")]
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
