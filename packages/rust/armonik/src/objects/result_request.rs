use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultRequest {
    pub session: String,
    pub result_id: String,
}

impl From<ResultRequest> for v3::ResultRequest {
    fn from(value: ResultRequest) -> Self {
        Self {
            session: value.session,
            result_id: value.result_id,
        }
    }
}

impl From<v3::ResultRequest> for ResultRequest {
    fn from(value: v3::ResultRequest) -> Self {
        Self {
            session: value.session,
            result_id: value.result_id,
        }
    }
}
