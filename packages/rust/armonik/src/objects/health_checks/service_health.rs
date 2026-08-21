use super::Status;

#[armonik_macros::message("armonik.api.grpc.v1.health_checks.CheckHealthResponse.ServiceHealth")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceHealth {
    pub name: String,
    /// Message.
    pub message: String,
    /// Health status.
    #[armonik(rename = "healthy")]
    pub health: Status,
}
