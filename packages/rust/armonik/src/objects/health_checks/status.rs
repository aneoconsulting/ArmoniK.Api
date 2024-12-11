use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Status {
    /// Unspecified.
    #[default]
    Unspecified = 0,
    /// Service is working without issues.
    Healthy = 1,
    /// Service has issues but still works.
    Degraded = 2,
    /// Service does not work.
    Unhealthy = 3,
}

impl From<i32> for Status {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Healthy,
            2 => Self::Degraded,
            3 => Self::Unhealthy,
            _ => Default::default(),
        }
    }
}

impl From<Status> for v3::health_checks::HealthStatusEnum {
    fn from(value: Status) -> Self {
        match value {
            Status::Unspecified => Self::Unspecified,
            Status::Healthy => Self::Healthy,
            Status::Degraded => Self::Degraded,
            Status::Unhealthy => Self::Unhealthy,
        }
    }
}

impl From<v3::health_checks::HealthStatusEnum> for Status {
    fn from(value: v3::health_checks::HealthStatusEnum) -> Self {
        match value {
            v3::health_checks::HealthStatusEnum::Unspecified => Self::Unspecified,
            v3::health_checks::HealthStatusEnum::Healthy => Self::Healthy,
            v3::health_checks::HealthStatusEnum::Degraded => Self::Degraded,
            v3::health_checks::HealthStatusEnum::Unhealthy => Self::Unhealthy,
        }
    }
}

super::super::impl_convert!(req Status : v3::health_checks::HealthStatusEnum);
