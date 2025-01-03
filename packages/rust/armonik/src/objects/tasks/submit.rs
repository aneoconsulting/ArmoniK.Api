use super::super::TaskOptions;

use crate::api::v3;

/// Task creation requests.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestItem {
    /// Unique ID of the results that will be produced by the task. Results should be created using ResultsService.
    pub expected_output_keys: Vec<String>,
    /// Unique ID of the results that will be used as data dependencies. Results should be created using ResultsService.
    pub data_dependencies: Vec<String>,
    /// Unique ID of the result that will be used as payload. Result should created using ResultsService.
    pub payload_id: String,
    /// Optional task options.
    pub task_options: Option<TaskOptions>,
}

super::super::impl_convert!(
    struct RequestItem = v3::tasks::submit_tasks_request::TaskCreation {
        expected_output_keys,
        data_dependencies,
        payload_id,
        option task_options,
    }
);

/// Request to create tasks.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// The session ID.
    pub session_id: String,
    /// The options for the tasks. Each task will have the same. Options are merged with the one from the session.
    pub task_options: Option<TaskOptions>,
    /// Task creation requests.
    pub items: Vec<RequestItem>,
}

super::super::impl_convert!(
    struct Request = v3::tasks::SubmitTasksRequest {
        session_id,
        option task_options,
        list items = list task_creations,
    }
);

/// task infos if submission successful, else throw gRPC exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseItem {
    /// The task ID.
    pub task_id: String,
    /// The expected output IDs. A task have expected output IDs.
    pub expected_output_ids: Vec<String>,
    /// The data dependencies IDs (inputs). A task have data dependencies.
    pub data_dependencies: Vec<String>,
    /// Unique ID of the result that will be used as payload.
    /// Result should created using ResultsService.
    pub payload_id: String,
}

super::super::impl_convert!(
    struct ResponseItem = v3::tasks::submit_tasks_response::TaskInfo {
        task_id,
        expected_output_ids,
        data_dependencies,
        payload_id,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub items: Vec<ResponseItem>,
}

super::super::impl_convert!(
    struct Response = v3::tasks::SubmitTasksResponse {
        list items = list task_infos,
    }
);
