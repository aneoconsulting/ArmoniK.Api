#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    Unknown(UnknownField),
}
