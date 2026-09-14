super::service! {
    Partitions in crate::partitions @ "armonik.api.grpc.v1.partitions.Partitions";

    rpc ListPartitions(list::Request) -> list::Response;
    rpc GetPartition(get::Request) -> get::Response;
}
