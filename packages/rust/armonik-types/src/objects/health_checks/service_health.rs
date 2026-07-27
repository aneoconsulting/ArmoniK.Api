use super::Status;

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.health_checks.CheckHealthResponse.ServiceHealth")]
pub struct ServiceHealth {
    /// Name of the service (e.g. "control_plane", "database", "redis").
    pub name: String,
    /// Message.
    pub message: String,
    /// Health status.
    #[armonik(rename = "healthy")]
    pub health: Status,
}
