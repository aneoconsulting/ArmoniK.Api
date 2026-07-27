use super::super::TaskOptions;

/// Task creation requests.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.SubmitTasksRequest.TaskCreation")]
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

/// Request to create tasks.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.SubmitTasksRequest")]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The session ID.
    pub session_id: String,
    /// The options for the tasks. Each task will have the same. Options are merged with the one from the session.
    pub task_options: Option<TaskOptions>,
    /// Task creation requests.
    #[armonik(rename = "task_creations")]
    pub items: Vec<RequestItem>,
}

/// task infos if submission successful, else throw gRPC exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.SubmitTasksResponse.TaskInfo")]
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

/// Response to create tasks.
///
/// expected_output_ids and data_dependencies must be created through ResultsService.
///
/// Remark : this may have to be enriched to a better management of errors but
/// will the client application be able to manage a missing data dependency or expected output ?
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.SubmitTasksResponse")]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// List of task infos if submission successful, else throw gRPC exception.
    #[armonik(rename = "task_infos")]
    pub items: Vec<ResponseItem>,
}
