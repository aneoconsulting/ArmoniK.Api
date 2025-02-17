use crate::api::v3;

use super::Status;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceHealth {
    /// Name of the service (e.g. "control_plane", "database", "redis").
    pub name: String,
    /// Message.
    pub message: String,
    /// Health status.
    pub health: Status,
}

super::super::impl_convert!(
    struct ServiceHealth = v3::health_checks::check_health_response::ServiceHealth {
        name,
        message,
        health = enum healthy,
    }
);
