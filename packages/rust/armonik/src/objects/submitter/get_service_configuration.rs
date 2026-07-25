use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

impl From<Request> for v3::Empty {
    fn from(_: Request) -> Self {
        Self {}
    }
}

impl From<v3::Empty> for Request {
    fn from(_: v3::Empty) -> Self {
        Self {}
    }
}

pub type Response = super::super::Configuration;
