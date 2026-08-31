pub use crate::rpc::health_checks::Client as HealthChecks;

use crate::client::client_method;

#[armonik_macros::client]
#[armonik(service = "armonik.api.grpc.v1.health_checks.HealthChecksService")]
impl<T: super::Channel> super::ServiceClient<crate::rpc::services::HealthChecks, T> {
    client_method!(CheckHealth:
        check()
        -> crate::health_checks::check::Request => services: Vec<crate::health_checks::ServiceHealth>);
}
