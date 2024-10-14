use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SessionStatus {
    #[default]
    Unspecified = 0, /* Session is in an unknown state. */
    Running = 1,   /* Session is open and accepting tasks for execution. */
    Cancelled = 2, /* Session is cancelled. No more tasks can be submitted. */
    Paused = 3, /* Session is paused. Tasks can be submitted but no more new tasks will be executed. Already running tasks will continue until they finish. */
    Closed = 4, /* Session is closed. No more tasks can be submitted and executed. */
    Purged = 5, /* Session is purged. No more tasks can be submitted and executed. Results data will be deleted. */
    Deleted = 6, /* Session is deleted. No more tasks can be submitted and executed. Sessions, tasks and results metadata associated to the session will be deleted. */
}

impl From<i32> for SessionStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Running,
            2 => Self::Cancelled,
            _ => Default::default(),
        }
    }
}

impl From<SessionStatus> for v3::session_status::SessionStatus {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Unspecified => Self::Unspecified,
            SessionStatus::Running => Self::Running,
            SessionStatus::Cancelled => Self::Cancelled,
            SessionStatus::Paused => Self::Paused,
            SessionStatus::Closed => Self::Closed,
            SessionStatus::Purged => Self::Purged,
            SessionStatus::Deleted => Self::Deleted,
        }
    }
}

impl From<v3::session_status::SessionStatus> for SessionStatus {
    fn from(value: v3::session_status::SessionStatus) -> Self {
        match value {
            v3::session_status::SessionStatus::Unspecified => Self::Unspecified,
            v3::session_status::SessionStatus::Running => Self::Running,
            v3::session_status::SessionStatus::Cancelled => Self::Cancelled,
            v3::session_status::SessionStatus::Paused => Self::Paused,
            v3::session_status::SessionStatus::Closed => Self::Closed,
            v3::session_status::SessionStatus::Purged => Self::Purged,
            v3::session_status::SessionStatus::Deleted => Self::Deleted,
        }
    }
}

super::impl_convert!(req SessionStatus : v3::session_status::SessionStatus);
