use std::sync::Arc;

use crate::health_checks;

/// The raw tonic server stub — the service trait and the tower service
/// wrapping an implementation of it — speaking the armonik types natively.
pub use crate::stubs::health_checks::health_checks_service_server as stub;

super::define_trait_methods! {
    trait HealthChecksService {
        /// Checks the health of the cluster. This can be used to verify that the cluster is up and running.
        fn health_checks::check;
    }
}

pub trait HealthChecksServiceExt {
    fn health_checks_server(self) -> stub::HealthChecksServiceServer<Self>
    where
        Self: Sized;
}

impl<T: HealthChecksService + Send + Sync + 'static> HealthChecksServiceExt for T {
    fn health_checks_server(self) -> stub::HealthChecksServiceServer<Self> {
        stub::HealthChecksServiceServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (stub::HealthChecksService) for HealthChecksService {
        fn check_health(crate::health_checks::check::Request) -> crate::health_checks::check::Response { check }
    }
}
