use super::super::TaskOptions;

use crate::api::v3;

/// Task creation requests.
#[derive(Debug, Clone, Default)]
pub struct RequestItem {
    /// Unique ID of the results that will be produced by the task. Results should be created using ResultsService.
    pub expected_output_keys: Vec<String>,
    /// Unique ID of the results that will be used as datadependencies. Results should be created using ResultsService.
    pub data_dependencies: Vec<String>,
    /// Unique ID of the result that will be used as payload. Result should created using ResultsService.
    pub payload_id: String,
    /// Optionnal task options.
    pub task_options: Option<TaskOptions>,
}

impl From<RequestItem> for v3::tasks::submit_tasks_request::TaskCreation {
    fn from(value: RequestItem) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
            task_options: value.task_options.map(Into::into),
        }
    }
}

impl From<v3::tasks::submit_tasks_request::TaskCreation> for RequestItem {
    fn from(value: v3::tasks::submit_tasks_request::TaskCreation) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
            task_options: value.task_options.map(Into::into),
        }
    }
}

super::super::impl_convert!(RequestItem : Option<v3::tasks::submit_tasks_request::TaskCreation>);

/// Request to create tasks.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// The session ID.
    pub session_id: String,
    /// The options for the tasks. Each task will have the same. Options are merged with the one from the session.
    pub task_options: Option<TaskOptions>,
    /// Task creation requests.
    pub items: Vec<RequestItem>,
}

impl From<Request> for v3::tasks::SubmitTasksRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            task_options: value.task_options.map(Into::into),
            task_creations: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::SubmitTasksRequest> for Request {
    fn from(value: v3::tasks::SubmitTasksRequest) -> Self {
        Self {
            session_id: value.session_id,
            task_options: value.task_options.map(Into::into),
            items: value.task_creations.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Request : Option<v3::tasks::SubmitTasksRequest>);

/// task infos if submission successful, else throw gRPC exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<ResponseItem> for v3::tasks::submit_tasks_response::TaskInfo {
    fn from(value: ResponseItem) -> Self {
        Self {
            task_id: value.task_id,
            expected_output_ids: value.expected_output_ids,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
        }
    }
}

impl From<v3::tasks::submit_tasks_response::TaskInfo> for ResponseItem {
    fn from(value: v3::tasks::submit_tasks_response::TaskInfo) -> Self {
        Self {
            task_id: value.task_id,
            expected_output_ids: value.expected_output_ids,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
        }
    }
}

super::super::impl_convert!(ResponseItem : Option<v3::tasks::submit_tasks_response::TaskInfo>);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub items: Vec<ResponseItem>,
}

impl From<Response> for v3::tasks::SubmitTasksResponse {
    fn from(value: Response) -> Self {
        Self {
            task_infos: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::SubmitTasksResponse> for Response {
    fn from(value: v3::tasks::SubmitTasksResponse) -> Self {
        Self {
            items: value.task_infos.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::tasks::SubmitTasksResponse>);
