use super::ServiceHealth;

#[armonik_macros::message("armonik.api.grpc.v1.health_checks.CheckHealthRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

#[armonik_macros::message("armonik.api.grpc.v1.health_checks.CheckHealthResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub services: Vec<ServiceHealth>,
}
