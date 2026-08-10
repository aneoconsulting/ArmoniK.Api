#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(enum = "armonik.api.grpc.v1.health_checks.HealthStatusEnum")]
pub enum Status {
    Healthy,
    Degraded,
    Unhealthy,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Unknown(UnknownStatus),
}
