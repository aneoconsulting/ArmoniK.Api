use std::sync::Arc;

use crate::applications;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::applications::applications_server as stub;

super::define_trait_methods! {
    trait ApplicationsService {
        fn applications::list;
    }
}

pub trait ApplicationsServiceExt {
    fn applications_server(self) -> stub::ApplicationsServer<Self>
    where
        Self: Sized;
}

impl<T: ApplicationsService + Send + Sync + 'static> ApplicationsServiceExt for T {
    fn applications_server(self) -> stub::ApplicationsServer<Self> {
        stub::ApplicationsServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::Applications) for ApplicationsService {
        fn list_applications(crate::applications::list::Request) -> crate::applications::list::Response { list }
    }
}
