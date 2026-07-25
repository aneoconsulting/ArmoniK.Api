use super::Raw;

/// Request to get a partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.GetPartitionRequest")]
pub struct Request {
    /// The partition ID.
    #[armonik(rename = "id")]
    pub partition_id: String,
}

/// Response to get a partition.
///
/// Return a raw partition.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.partitions.GetPartitionResponse")]
pub struct Response {
    /// The raw partition.
    pub partition: Raw,
}
