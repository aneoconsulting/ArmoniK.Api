use std::sync::Arc;

use crate::api::v3;
use crate::partitions;

super::define_trait_methods! {
    trait PartitionsService {
        fn partitions::list;
        fn partitions::get;
    }
}

pub trait PartitionsServiceExt {
    fn partitions_server(self) -> v3::partitions::partitions_server::PartitionsServer<Self>
    where
        Self: Sized;
}

impl<T: PartitionsService + Send + Sync + 'static> PartitionsServiceExt for T {
    fn partitions_server(self) -> v3::partitions::partitions_server::PartitionsServer<Self> {
        v3::partitions::partitions_server::PartitionsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::partitions::partitions_server::Partitions) for PartitionsService {
        fn list_partitions(crate::partitions::list::Request) -> crate::partitions::list::Response { list }
        fn get_partition(crate::partitions::get::Request) -> crate::partitions::get::Response { get }
    }
}
