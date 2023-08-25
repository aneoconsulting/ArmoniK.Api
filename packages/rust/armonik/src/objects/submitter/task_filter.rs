use super::super::TaskStatus;
use crate::utils::IntoCollection;

use crate::api::v3;

#[derive(Debug, Clone)]
pub enum TaskFilterIds {
    Sessions(Vec<String>),
    Tasks(Vec<String>),
}

impl Default for TaskFilterIds {
    fn default() -> Self {
        Self::Sessions(Default::default())
    }
}

impl From<TaskFilterIds> for v3::submitter::task_filter::Ids {
    fn from(value: TaskFilterIds) -> Self {
        match value {
            TaskFilterIds::Sessions(sessions) => {
                Self::Session(v3::submitter::task_filter::IdsRequest { ids: sessions })
            }
            TaskFilterIds::Tasks(tasks) => {
                Self::Task(v3::submitter::task_filter::IdsRequest { ids: tasks })
            }
        }
    }
}

impl From<v3::submitter::task_filter::Ids> for TaskFilterIds {
    fn from(value: v3::submitter::task_filter::Ids) -> Self {
        match value {
            v3::submitter::task_filter::Ids::Session(sessions) => Self::Sessions(sessions.ids),
            v3::submitter::task_filter::Ids::Task(tasks) => Self::Tasks(tasks.ids),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskFilterStatuses {
    Include(Vec<TaskStatus>),
    Exclude(Vec<TaskStatus>),
}

impl Default for TaskFilterStatuses {
    fn default() -> Self {
        Self::Exclude(Default::default())
    }
}

impl From<TaskFilterStatuses> for v3::submitter::task_filter::Statuses {
    fn from(value: TaskFilterStatuses) -> Self {
        match value {
            TaskFilterStatuses::Include(statuses) => {
                Self::Excluded(v3::submitter::task_filter::StatusesRequest {
                    statuses: statuses.into_iter().map(|status| status as i32).collect(),
                })
            }
            TaskFilterStatuses::Exclude(statuses) => {
                Self::Included(v3::submitter::task_filter::StatusesRequest {
                    statuses: statuses.into_iter().map(|status| status as i32).collect(),
                })
            }
        }
    }
}

impl From<v3::submitter::task_filter::Statuses> for TaskFilterStatuses {
    fn from(value: v3::submitter::task_filter::Statuses) -> Self {
        match value {
            v3::submitter::task_filter::Statuses::Excluded(statuses) => {
                Self::Exclude(statuses.statuses.into_collect())
            }
            v3::submitter::task_filter::Statuses::Included(statuses) => {
                Self::Include(statuses.statuses.into_collect())
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub ids: TaskFilterIds,
    pub statuses: TaskFilterStatuses,
}

super::super::impl_convert!(
    struct TaskFilter = v3::submitter::TaskFilter {
        ids = option ids,
        statuses = option statuses,
    }
);
