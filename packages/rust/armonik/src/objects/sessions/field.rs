use super::super::TaskOptionField;

#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent, message = "armonik.api.grpc.v1.sessions.SessionRawField")]
pub enum RawField {
    SessionId,
    Status,
    ClientSubmission,
    WorkerSubmission,
    PartitionIds,
    Options,
    CreatedAt,
    CancelledAt,
    ClosedAt,
    PurgedAt,
    DeletedAt,
    Duration,
    /// Unspecified (zero) or a field unknown to this crate version.
    Unknown(UnknownRawField),
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.sessions.SessionField")]
pub enum Field {
    /// No field: the oneof was left unset.
    ///
    /// Distinct from a raw field holding the unspecified value, which is what the absence used to
    /// decode to: both name no field, but only this one round-trips as the empty message.
    #[default]
    Invalid,
    /// The session raw field.
    #[armonik(rename = "session_raw_field")]
    Raw(RawField),
    #[armonik(rename = "task_option_field")]
    TaskOption(TaskOptionField),
    #[armonik(
        rename = "task_option_generic_field",
        with = "crate::codec::adapters::Wrapper",
        absorbs = "armonik.api.grpc.v1.sessions.TaskOptionGenericField"
    )]
    TaskOptionGeneric(String),
}
