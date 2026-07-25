use crate::api::v3;

/// Request for cancelling tasks, standing for the `TaskFilter` message the
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

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {}

impl From<Response> for v3::Empty {
    fn from(_: Response) -> Self {
        Self {}
    }
}

impl From<v3::Empty> for Response {
    fn from(_: v3::Empty) -> Self {
        Self {}
    }
}
