use std::collections::HashMap;

use crate::api::v3;

/// Request for getting result ids of tasks ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The task IDs.
    pub task_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::tasks::GetResultIdsRequest {
        list task_ids = list task_id,
    }
);

/// Response for getting result ids of tasks ids.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The task results.
    pub task_results: HashMap<String, Vec<String>>,
}

impl From<Response> for v3::tasks::GetResultIdsResponse {
    fn from(value: Response) -> Self {
        Self {
            task_results: value
                .task_results
                .into_iter()
                .map(
                    |(task_id, result_ids)| v3::tasks::get_result_ids_response::MapTaskResult {
                        task_id,
                        result_ids,
                    },
                )
                .collect(),
        }
    }
}

impl From<v3::tasks::GetResultIdsResponse> for Response {
    fn from(value: v3::tasks::GetResultIdsResponse) -> Self {
        Self {
            task_results: value
                .task_results
                .into_iter()
                .map(|pair| (pair.task_id, pair.result_ids))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::tasks::GetResultIdsResponse);
