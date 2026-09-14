pub use crate::rpc::partitions::Client as Partitions;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.partitions.Partitions")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::Partitions, T> {
    client_method!(ListPartitions:
        list(filters: filters<crate::partitions::filter::Field>, sort: plain<crate::partitions::Sort>, page: plain<i32>, page_size: plain<i32>)
        -> crate::partitions::list::Request => crate::partitions::list::Response);

    client_method!(GetPartition:
        get(partition_id: into<String>)
        -> crate::partitions::get::Request => partition: crate::partitions::Raw);
}
