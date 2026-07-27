use std::sync::Arc;

use crate::versions;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::versions::versions_server as stub;

super::define_trait_methods! {
    trait VersionsService {
        fn versions::list;
    }
}

pub trait VersionsServiceExt {
    fn versions_server(self) -> stub::VersionsServer<Self>
    where
        Self: Sized;
}

impl<T: VersionsService + Send + Sync + 'static> VersionsServiceExt for T {
    fn versions_server(self) -> stub::VersionsServer<Self> {
        stub::VersionsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Versions) for VersionsService {
        fn list_versions(crate::versions::list::Request) -> crate::versions::list::Response { list }
    }
}
