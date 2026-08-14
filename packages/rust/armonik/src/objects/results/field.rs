#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent, message = "armonik.api.grpc.v1.results.ResultField")]
pub enum Field {
    SessionId,
    Name,
    OwnerTaskId,
    Status,
    CreatedAt,
    CompletedAt,
    ResultId,
    Size,
    CreatedBy,
    OpaqueId,
    ManualDeletion,
    /// Unspecified (zero) or a field unknown to this crate version.
    Unknown(UnknownField),
}
