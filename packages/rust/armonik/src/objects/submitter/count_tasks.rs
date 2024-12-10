use crate::api::v3;

#[derive(Debug, Clone, Default)]
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

pub type Response = super::super::Count;
