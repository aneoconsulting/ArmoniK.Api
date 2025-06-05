use std::sync::Arc;

use crate::api::v3;
use crate::versions;

super::define_trait_methods! {
    trait VersionsService {
        fn versions::list;
    }
}

pub trait VersionsServiceExt {
    fn versions_server(self) -> v3::versions::versions_server::VersionsServer<Self>
    where
        Self: Sized;
}

impl<T: VersionsService + Send + Sync + 'static> VersionsServiceExt for T {
    fn versions_server(self) -> v3::versions::versions_server::VersionsServer<Self> {
        v3::versions::versions_server::VersionsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::versions::versions_server::Versions) for VersionsService {
        fn list_versions(v3::versions::ListVersionsRequest) -> v3::versions::ListVersionsResponse { list }
    }
}
