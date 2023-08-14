use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct ResultIds {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

pub type DeleteResultsDataRequest = ResultIds;
pub type DeleteResultsDataResponse = ResultIds;

impl From<ResultIds> for v3::results::DeleteResultsDataRequest {
    fn from(value: ResultIds) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_ids,
        }
    }
}

impl From<ResultIds> for v3::results::DeleteResultsDataResponse {
    fn from(value: ResultIds) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_ids,
        }
    }
}

impl From<v3::results::DeleteResultsDataRequest> for ResultIds {
    fn from(value: v3::results::DeleteResultsDataRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_ids: value.result_id,
        }
    }
}

impl From<v3::results::DeleteResultsDataResponse> for ResultIds {
    fn from(value: v3::results::DeleteResultsDataResponse) -> Self {
        Self {
            session_id: value.session_id,
            result_ids: value.result_id,
        }
    }
}

super::super::impl_convert!(ResultIds : Option<v3::results::DeleteResultsDataRequest>);
super::super::impl_convert!(ResultIds : Option<v3::results::DeleteResultsDataResponse>);
