use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub filter: super::SessionFilter,
}

impl From<Request> for v3::submitter::SessionFilter {
    fn from(value: Request) -> Self {
        value.filter.into()
    }
}

impl From<v3::submitter::SessionFilter> for Request {
    fn from(value: v3::submitter::SessionFilter) -> Self {
        Self {
            filter: value.into(),
        }
    }
}

super::super::impl_convert!(req Request : v3::submitter::SessionFilter);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub session_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Response = v3::submitter::SessionIdList {
        list session_ids,
    }
);
