use super::super::TaskOptions;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksRequest.TaskCreation")]
pub struct RequestItem {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload_id: String,
    pub task_options: Option<TaskOptions>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksRequest")]
pub struct Request {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
    #[armonik(rename = "task_creations")]
    pub items: Vec<RequestItem>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksResponse.TaskInfo")]
pub struct ResponseItem {
    pub task_id: String,
    pub expected_output_ids: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.SubmitTasksResponse")]
pub struct Response {
    #[armonik(rename = "task_infos")]
    pub items: Vec<ResponseItem>,
}
