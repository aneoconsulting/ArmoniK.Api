/// Request for listing sessions, standing for the `SessionFilter` message
/// the stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub filter: super::SessionFilter,
}

impl From<Request> for super::SessionFilter {
    fn from(value: Request) -> Self {
        value.filter
    }
}

impl From<super::SessionFilter> for Request {
    fn from(value: super::SessionFilter) -> Self {
        Self { filter: value }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.SessionIdList")]
pub struct Response {
    pub session_ids: Vec<String>,
}
