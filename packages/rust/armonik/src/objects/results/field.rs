#[armonik_macros::enumeration("armonik.api.grpc.v1.results.ResultField")]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent)]
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
