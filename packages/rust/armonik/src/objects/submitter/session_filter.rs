use super::super::SessionStatus;
use crate::utils::IntoCollection;

use crate::api::v3;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SessionFilterStatuses {
    Include(Vec<SessionStatus>),
    Exclude(Vec<SessionStatus>),
}

impl Default for SessionFilterStatuses {
    fn default() -> Self {
        Self::Exclude(Default::default())
    }
}

impl From<SessionFilterStatuses> for v3::submitter::session_filter::Statuses {
    fn from(value: SessionFilterStatuses) -> Self {
        match value {
            SessionFilterStatuses::Include(statuses) => {
                Self::Excluded(v3::submitter::session_filter::StatusesRequest {
                    statuses: statuses.into_iter().map(|status| status as i32).collect(),
                })
            }
            SessionFilterStatuses::Exclude(statuses) => {
                Self::Included(v3::submitter::session_filter::StatusesRequest {
                    statuses: statuses.into_iter().map(|status| status as i32).collect(),
                })
            }
        }
    }
}

impl From<v3::submitter::session_filter::Statuses> for SessionFilterStatuses {
    fn from(value: v3::submitter::session_filter::Statuses) -> Self {
        match value {
            v3::submitter::session_filter::Statuses::Excluded(statuses) => {
                Self::Exclude(statuses.statuses.into_collect())
            }
            v3::submitter::session_filter::Statuses::Included(statuses) => {
                Self::Include(statuses.statuses.into_collect())
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionFilter {
    pub ids: Vec<String>,
    pub statuses: SessionFilterStatuses,
}

super::super::impl_convert!(
    struct SessionFilter = v3::submitter::SessionFilter {
        list ids = list sessions,
        statuses = option statuses,
    }
);
