#[armonik_macros::enumeration("armonik.api.grpc.v1.applications.ApplicationField")]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent)]
pub enum Field {
    Name,
    Version,
    Namespace,
    Service,
    /// Unspecified (zero) or a field unknown to this crate version.
    Unknown(UnknownField),
}
