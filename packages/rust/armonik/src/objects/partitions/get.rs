use super::Raw;

#[armonik_macros::message("armonik.api.grpc.v1.partitions.GetPartitionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    #[armonik(rename = "id")]
    pub partition_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.partitions.GetPartitionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub partition: Raw,
}
