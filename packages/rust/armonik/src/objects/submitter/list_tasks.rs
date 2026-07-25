/// Request for listing tasks, standing for the `TaskFilter` message the
/// stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub filter: super::TaskFilter,
}

impl From<Request> for super::TaskFilter {
    fn from(value: Request) -> Self {
        value.filter
    }
}

impl From<super::TaskFilter> for Request {
    fn from(value: super::TaskFilter) -> Self {
        Self { filter: value }
    }
}

/// Response for listing tasks, standing for the `TaskIdList` message the
/// stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub task_ids: Vec<String>,
}

impl From<Response> for crate::TaskIdList {
    fn from(value: Response) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}

impl From<crate::TaskIdList> for Response {
    fn from(value: crate::TaskIdList) -> Self {
        Self {
            task_ids: value.task_ids,
        }
    }
}
