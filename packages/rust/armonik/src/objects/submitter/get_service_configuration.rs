#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

impl From<Request> for crate::Empty {
    fn from(_: Request) -> Self {
        Self {}
    }
}

impl From<crate::Empty> for Request {
    fn from(_: crate::Empty) -> Self {
        Self {}
    }
}

pub type Response = super::super::Configuration;
