use super::super::TaskOptionField;

/// Represents every available field in a session raw.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.sessions.SessionRawField")]
pub enum RawField {
    /// The session ID.
    #[default]
    SessionId,
    /// The session status.
    Status,
    /// Whether clients can submit tasks in the session.
    ClientSubmission,
    /// Whether workers can submit tasks in the session.
    WorkerSubmission,
    /// The partition IDs.
    PartitionIds,
    /// The task options. In fact, these are used as default value in child tasks.
    Options,
    /// The creation date.
    CreatedAt,
    /// The cancellation date. Only set when status is 'cancelled'.
    CancelledAt,
    /// The closure date. Only set when status is 'closed'.
    ClosedAt,
    /// The purge date. Only set when status is 'purged'.
    PurgedAt,
    /// The deletion date. Only set when status is 'deleted'.
    DeletedAt,
    /// The duration. Only set when status is 'cancelled' and 'closed'.
    Duration,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherRawField),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.SessionField", oneof = "field")]
pub enum Field {
    /// The session raw field.
    #[armonik(rename = "session_raw_field")]
    Raw(RawField),
    /// The task option field.
    #[armonik(rename = "task_option_field")]
    TaskOption(TaskOptionField),
    /// Represents a generic field in a task option.
    #[armonik(
        rename = "task_option_generic_field",
        with = "crate::codec::adapters::StringWrapper<1>"
    )]
    TaskOptionGeneric(String),
}

impl Default for Field {
    fn default() -> Self {
        Self::Raw(Default::default())
    }
}
