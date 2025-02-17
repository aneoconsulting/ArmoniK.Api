use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub filter: super::TaskFilter,
}

impl From<Request> for v3::submitter::TaskFilter {
    fn from(value: Request) -> Self {
        value.filter.into()
    }
}

impl From<v3::submitter::TaskFilter> for Request {
    fn from(value: v3::submitter::TaskFilter) -> Self {
        Self {
            filter: value.into(),
        }
    }
}

super::super::impl_convert!(req Request : v3::submitter::TaskFilter);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {}

super::super::impl_convert!(
    struct Response = v3::Empty {
    }
);
