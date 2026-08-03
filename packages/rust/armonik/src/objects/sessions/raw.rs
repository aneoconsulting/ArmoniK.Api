use super::super::{SessionStatus, TaskOptions};

/// A raw session object.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.SessionRaw")]
pub struct Raw {
    /// The session ID.
    pub session_id: String,
    /// The session status.
    pub status: SessionStatus,
    /// Whether clients can submit tasks in the session.
    pub client_submission: bool,
    /// Whether workers can submit tasks in the session.
    pub worker_submission: bool,
    /// The partition IDs.
    pub partition_ids: Vec<String>,
    /// The task options. In fact, these are used as default value in child tasks.
    #[armonik(rename = "options")]
    pub default_task_options: TaskOptions,
    /// The creation date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
    /// The cancellation date. Only set when status is 'cancelled'.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub cancelled_at: Option<prost_types::Timestamp>,
    /// The closure date. Only set when status is 'closed'.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub closed_at: Option<prost_types::Timestamp>,
    /// The purge date. Only set when status is 'purged'.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub purged_at: Option<prost_types::Timestamp>,
    /// The deletion date. Only set when status is 'deleted'.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub deleted_at: Option<prost_types::Timestamp>,
    /// The duration. Only set when status is 'cancelled'.
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub duration: Option<prost_types::Duration>,
}
