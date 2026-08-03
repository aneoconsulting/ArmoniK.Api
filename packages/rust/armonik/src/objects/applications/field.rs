/// Represents every available field in a Application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    transparent,
    message = "armonik.api.grpc.v1.applications.ApplicationField"
)]
pub enum Field {
    /// Application name.
    Name,
    /// Application version.
    Version,
    /// Application namespace.
    Namespace,
    /// Application service.
    Service,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherField),
}
