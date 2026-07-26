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

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
pub struct Response {}
