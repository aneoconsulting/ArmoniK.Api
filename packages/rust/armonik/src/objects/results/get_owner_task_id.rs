use std::collections::{HashMap, HashSet};

use crate::api::v3;

/// Request for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetOwnerTaskIdRequest {
    /// The session ID.
    pub session_id: String,
    /// The list of result ID/name.
    pub result_ids: HashSet<String>,
}

impl From<GetOwnerTaskIdRequest> for v3::results::GetOwnerTaskIdRequest {
    fn from(value: GetOwnerTaskIdRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_ids.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::GetOwnerTaskIdRequest> for GetOwnerTaskIdRequest {
    fn from(value: v3::results::GetOwnerTaskIdRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_ids: value.result_id.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(GetOwnerTaskIdRequest : Option<v3::results::GetOwnerTaskIdRequest>);

/// Response for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetOwnerTaskIdResponse {
    /// Map to get the owner task id for each result id.
    pub result_task: HashMap<String, String>,
    /// The session ID.
    pub session_id: String,
}

impl From<GetOwnerTaskIdResponse> for v3::results::GetOwnerTaskIdResponse {
    fn from(value: GetOwnerTaskIdResponse) -> Self {
        Self {
            result_task: value
                .result_task
                .into_iter()
                .map(
                    |(key, value)| v3::results::get_owner_task_id_response::MapResultTask {
                        result_id: key,
                        task_id: value,
                    },
                )
                .collect(),
            session_id: value.session_id,
        }
    }
}

impl From<v3::results::GetOwnerTaskIdResponse> for GetOwnerTaskIdResponse {
    fn from(value: v3::results::GetOwnerTaskIdResponse) -> Self {
        Self {
            result_task: value
                .result_task
                .into_iter()
                .map(|key_value_pair| (key_value_pair.result_id, key_value_pair.task_id))
                .collect(),
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(GetOwnerTaskIdResponse : Option<v3::results::GetOwnerTaskIdResponse>);
