use std::sync::Arc;

use crate::api::v3;
use crate::health_checks;

super::define_trait_methods! {
    trait HealthChecksService {
        /// Checks the health of the cluster. This can be used to verify that the cluster is up and running.
        fn health_checks::check;
    }
}

pub trait HealthChecksServiceExt {
    fn health_checks_server(
        self,
    ) -> v3::health_checks::health_checks_service_server::HealthChecksServiceServer<Self>
    where
        Self: Sized;
}

impl<T: HealthChecksService + Send + Sync + 'static> HealthChecksServiceExt for T {
    fn health_checks_server(
        self,
    ) -> v3::health_checks::health_checks_service_server::HealthChecksServiceServer<Self> {
        v3::health_checks::health_checks_service_server::HealthChecksServiceServer::new(self)
    }
}

super::impl_trait_methods! {
    impl (v3::health_checks::health_checks_service_server::HealthChecksService) for HealthChecksService {
        fn check_health(v3::health_checks::CheckHealthRequest) -> v3::health_checks::CheckHealthResponse { check }
    }
}
