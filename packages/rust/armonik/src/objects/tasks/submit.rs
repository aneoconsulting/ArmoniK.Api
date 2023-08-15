use super::super::TaskOptions;

use crate::api::v3;

/// Task creation requests.
#[derive(Debug, Clone, Default)]
pub struct CreationRequest {
    /// Unique ID of the results that will be produced by the task. Results should be created using ResultsService.
    pub expected_output_keys: Vec<String>,
    /// Unique ID of the results that will be used as datadependencies. Results should be created using ResultsService.
    pub data_dependencies: Vec<String>,
    /// Unique ID of the result that will be used as payload. Result should created using ResultsService.
    pub payload_id: String,
    /// Optionnal task options.
    pub task_options: Option<TaskOptions>,
}

impl From<CreationRequest> for v3::tasks::submit_tasks_request::TaskCreation {
    fn from(value: CreationRequest) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
            task_options: value.task_options.map(Into::into),
        }
    }
}

impl From<v3::tasks::submit_tasks_request::TaskCreation> for CreationRequest {
    fn from(value: v3::tasks::submit_tasks_request::TaskCreation) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
            task_options: value.task_options.map(Into::into),
        }
    }
}

super::super::impl_convert!(CreationRequest : Option<v3::tasks::submit_tasks_request::TaskCreation>);

/// Request to create tasks.
#[derive(Debug, Clone, Default)]
pub struct SubmitTasksRequest {
    /// The session ID.
    pub session_id: String,
    /// The options for the tasks. Each task will have the same. Options are merged with the one from the session.
    pub task_options: Option<TaskOptions>,
    /// Task creation requests.
    pub task_creations: Vec<CreationRequest>,
}

impl From<SubmitTasksRequest> for v3::tasks::SubmitTasksRequest {
    fn from(value: SubmitTasksRequest) -> Self {
        Self {
            session_id: value.session_id,
            task_options: value.task_options.map(Into::into),
            task_creations: value.task_creations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::SubmitTasksRequest> for SubmitTasksRequest {
    fn from(value: v3::tasks::SubmitTasksRequest) -> Self {
        Self {
            session_id: value.session_id,
            task_options: value.task_options.map(Into::into),
            task_creations: value.task_creations.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(SubmitTasksRequest : Option<v3::tasks::SubmitTasksRequest>);

/// task infos if submission successful, else throw gRPC exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskInfo {
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

impl From<TaskInfo> for v3::tasks::submit_tasks_response::TaskInfo {
    fn from(value: TaskInfo) -> Self {
        Self {
            task_id: value.task_id,
            expected_output_ids: value.expected_output_ids,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
        }
    }
}

impl From<v3::tasks::submit_tasks_response::TaskInfo> for TaskInfo {
    fn from(value: v3::tasks::submit_tasks_response::TaskInfo) -> Self {
        Self {
            task_id: value.task_id,
            expected_output_ids: value.expected_output_ids,
            data_dependencies: value.data_dependencies,
            payload_id: value.payload_id,
        }
    }
}

super::super::impl_convert!(TaskInfo : Option<v3::tasks::submit_tasks_response::TaskInfo>);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubmitTasksResponse {
    pub task_infos: Vec<TaskInfo>,
}

impl From<SubmitTasksResponse> for v3::tasks::SubmitTasksResponse {
    fn from(value: SubmitTasksResponse) -> Self {
        Self {
            task_infos: value.task_infos.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::SubmitTasksResponse> for SubmitTasksResponse {
    fn from(value: v3::tasks::SubmitTasksResponse) -> Self {
        Self {
            task_infos: value.task_infos.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(SubmitTasksResponse : Option<v3::tasks::SubmitTasksResponse>);
