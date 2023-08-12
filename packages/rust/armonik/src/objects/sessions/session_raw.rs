use super::super::{SessionStatus, TaskOptions};

use crate::api::v3;

/// A raw session object.
#[derive(Debug, Clone, Default)]
pub struct SessionRaw {
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

impl From<SessionRaw> for v3::sessions::SessionRaw {
    fn from(value: SessionRaw) -> Self {
        Self {
            session_id: value.session_id,
            status: value.status as i32,
            partition_ids: value.partition_ids,
            options: value.options.into(),
            created_at: value.created_at,
            cancelled_at: value.cancelled_at,
            duration: value.duration,
        }
    }
}

impl From<v3::sessions::SessionRaw> for SessionRaw {
    fn from(value: v3::sessions::SessionRaw) -> Self {
        Self {
            session_id: value.session_id,
            status: value.status.into(),
            partition_ids: value.partition_ids,
            options: value.options.into(),
            created_at: value.created_at,
            cancelled_at: value.cancelled_at,
            duration: value.duration,
        }
    }
}

super::super::impl_convert!(SessionRaw : Option<v3::sessions::SessionRaw>);
