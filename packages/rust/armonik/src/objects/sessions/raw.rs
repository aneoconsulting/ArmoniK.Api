use super::super::{SessionStatus, TaskOptions};

use crate::api::v3;

/// A raw session object.
#[derive(Debug, Clone, Default)]
pub struct Raw {
    /// The session ID.
    pub session_id: String,
    /// The session status.
    pub status: SessionStatus,
    /// The partition IDs.
    pub partition_ids: Vec<String>,
    /// The task options. In fact, these are used as default value in child tasks.
    pub options: TaskOptions,
    /// The creation date.
    pub created_at: Option<prost_types::Timestamp>,
    /// The cancellation date. Only set when status is 'cancelled'.
    pub cancelled_at: Option<prost_types::Timestamp>,
    /// The duration. Only set when status is 'cancelled'.
    pub duration: Option<prost_types::Duration>,
}

super::super::impl_convert!(
    struct Raw = v3::sessions::SessionRaw {
        session_id,
        status = enum status,
        partition_ids,
        options = option options,
        created_at,
        cancelled_at,
        duration,
    }
);
