use super::Status;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.health_checks.CheckHealthResponse.ServiceHealth")]
pub struct ServiceHealth {
    pub name: String,
    /// Message.
    pub message: String,
    /// Health status.
    #[armonik(rename = "healthy")]
    pub health: Status,
}
