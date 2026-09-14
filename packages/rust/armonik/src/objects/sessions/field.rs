use super::super::TaskOptionField;

#[armonik_macros::enumeration("armonik.api.grpc.v1.sessions.SessionRawField")]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent)]
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

#[armonik_macros::message("armonik.api.grpc.v1.sessions.SessionField")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// No field named. `Raw(RawField::UNSPECIFIED)` says the same thing one level down, but only
    /// this one round-trips as the empty message.
    #[default]
    Invalid,
    /// The session raw field.
    #[armonik(rename = "session_raw_field")]
    Raw(RawField),
    #[armonik(rename = "task_option_field")]
    TaskOption(TaskOptionField),
    #[armonik(rename = "task_option_generic_field", inlined)]
    TaskOptionGeneric(String),
}
