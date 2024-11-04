use std::collections::HashMap;

use crate::api::v3;

/// Request for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The session ID.
    pub session_id: String,
    /// The list of result ID/name.
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = crate::api::v3::results::GetOwnerTaskIdRequest {
        session_id,
        list result_ids = list result_id,
    }
);

/// Response for getting the id of the task that should create this result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// Map to get the owner task id for each result id.
    pub result_task: HashMap<String, String>,
    /// The session ID.
    pub session_id: String,
}

impl From<Response> for v3::results::GetOwnerTaskIdResponse {
    fn from(value: Response) -> Self {
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

impl From<v3::results::GetOwnerTaskIdResponse> for Response {
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

super::super::impl_convert!(req Response : v3::results::GetOwnerTaskIdResponse);
