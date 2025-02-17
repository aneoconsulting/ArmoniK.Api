use super::super::{SessionStatus, TaskOptions};

use crate::api::v3;

/// A raw session object.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

super::super::impl_convert!(
    struct Raw = v3::sessions::SessionRaw {
        session_id,
        status = enum status,
        client_submission,
        worker_submission,
        partition_ids,
        default_task_options = option options,
        created_at,
        cancelled_at,
        closed_at,
        purged_at,
        deleted_at,
        duration,
    }
);
