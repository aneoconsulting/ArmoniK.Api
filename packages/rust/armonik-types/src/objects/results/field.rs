/// Represents every available field in a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.results.ResultField")]
pub enum Field {
    /// The session ID.
    SessionId,
    /// The result name.
    Name,
    /// The owner task ID.
    OwnerTaskId,
    /// The result status.
    Status,
    /// The result creation date.
    CreatedAt,
    /// The result completion date.
    CompletedAt,
    /// The result ID.
    ResultId,
    /// The size of the result.
    Size,
    /// The ID of the Task that as submitted this result.
    CreatedBy,
    /// The ID of the data in the underlying object storage.
    OpaqueId,
    /// If the user is responsible for the deletion of the data in the underlying object storage.
    ManualDeletion,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherField),
}
