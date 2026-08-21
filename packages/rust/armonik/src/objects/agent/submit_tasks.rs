use super::super::TaskOptions;

#[armonik_macros::message("armonik.api.grpc.v1.agent.SubmitTasksRequest.TaskCreation")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestItem {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload_id: String,
    pub task_options: Option<TaskOptions>,
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.SubmitTasksRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub communication_token: String,
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
    #[armonik(rename = "task_creations")]
    pub items: Vec<RequestItem>,
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.SubmitTasksResponse.TaskInfo")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResponseItem {
    pub task_id: String,
    pub expected_output_ids: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload_id: String,
}

#[armonik_macros::message("armonik.api.grpc.v1.agent.SubmitTasksResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub communication_token: String,
    #[armonik(rename = "task_infos")]
    pub items: Vec<ResponseItem>,
}
