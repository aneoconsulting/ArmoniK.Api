use std::sync::Arc;

use crate::partitions;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::partitions::partitions_server as stub;

super::define_trait_methods! {
    trait PartitionsService {
        fn partitions::list;
        fn partitions::get;
    }
}

pub trait PartitionsServiceExt {
    fn partitions_server(self) -> stub::PartitionsServer<Self>
    where
        Self: Sized;
}

impl<T: PartitionsService + Send + Sync + 'static> PartitionsServiceExt for T {
    fn partitions_server(self) -> stub::PartitionsServer<Self> {
        stub::PartitionsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Partitions) for PartitionsService {
        fn list_partitions(crate::partitions::list::Request) -> crate::partitions::list::Response { list }
        fn get_partition(crate::partitions::get::Request) -> crate::partitions::get::Response { get }
    }
}
