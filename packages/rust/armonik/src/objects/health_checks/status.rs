#[armonik_macros::enumeration("armonik.api.grpc.v1.health_checks.HealthStatusEnum")]
#[derive(Debug, Clone, Copy)]
pub enum Status {
    Healthy,
    Degraded,
    Unhealthy,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Unknown(UnknownStatus),
}
