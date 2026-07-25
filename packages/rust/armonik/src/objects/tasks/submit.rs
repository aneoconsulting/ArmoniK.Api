use super::super::TaskOptions;

/// Task creation requests.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksRequest.TaskCreation")]
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
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksRequest")]
pub struct Request {
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
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksResponse.TaskInfo")]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksResponse")]
pub struct Response {
    /// List of task infos if submission successful, else throw gRPC exception.
    #[armonik(rename = "task_infos")]
    pub items: Vec<ResponseItem>,
}
