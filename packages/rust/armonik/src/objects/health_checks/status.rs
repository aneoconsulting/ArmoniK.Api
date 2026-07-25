use crate::api::v3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.health_checks.HealthStatusEnum")]
pub enum Status {
    /// Service is working without issues.
    Healthy,
    /// Service has issues but still works.
    Degraded,
    /// Service does not work.
    Unhealthy,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherStatus),
}

impl From<Status> for v3::health_checks::HealthStatusEnum {
    fn from(value: Status) -> Self {
        Self::try_from(i32::from(value)).unwrap_or(Self::Unspecified)
    }
}

impl From<v3::health_checks::HealthStatusEnum> for Status {
    fn from(value: v3::health_checks::HealthStatusEnum) -> Self {
        Self::from(value as i32)
    }
}
