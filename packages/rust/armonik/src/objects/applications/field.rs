#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    transparent,
    message = "armonik.api.grpc.v1.applications.ApplicationField"
)]
pub enum Field {
    Name,
    Version,
    Namespace,
    Service,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherField),
}
