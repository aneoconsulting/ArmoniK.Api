use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SessionStatus {
    #[default]
    Unspecified = 0, /* Session is in an unknown state. */
    Running = 1,   /* Session is open and accepting tasks for execution. */
    Cancelled = 2, /* Session is cancelled. No more tasks can be submitted. */
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
        }
    }
}

impl From<v3::session_status::SessionStatus> for SessionStatus {
    fn from(value: v3::session_status::SessionStatus) -> Self {
        match value {
            v3::session_status::SessionStatus::Unspecified => Self::Unspecified,
            v3::session_status::SessionStatus::Running => Self::Running,
            v3::session_status::SessionStatus::Cancelled => Self::Cancelled,
        }
    }
}
