use super::super::{SessionStatus, TaskOptions};

use crate::api::v3;

/// A raw session object.
#[derive(Debug, Clone, Default)]
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
    pub created_at: Option<prost_types::Timestamp>,
    /// The cancellation date. Only set when status is 'cancelled'.
    pub cancelled_at: Option<prost_types::Timestamp>,
    /// The closure date. Only set when status is 'closed'.
    pub closed_at: Option<prost_types::Timestamp>,
    /// The purge date. Only set when status is 'purged'.
    pub purged_at: Option<prost_types::Timestamp>,
    /// The deletion date. Only set when status is 'deleted'.
    pub deleted_at: Option<prost_types::Timestamp>,
    /// The duration. Only set when status is 'cancelled'.
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
