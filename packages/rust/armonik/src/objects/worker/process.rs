use super::super::{Configuration, Output, TaskOptions};

#[armonik_macros::message("armonik.api.grpc.v1.worker.ProcessRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub communication_token: String,
    pub session_id: String,
    pub task_id: String,
    pub task_options: TaskOptions,
    pub expected_output_keys: Vec<String>,
    pub payload_id: String,
    pub data_dependencies: Vec<String>,
    pub data_folder: String,
    pub configuration: Configuration,
}

#[armonik_macros::message("armonik.api.grpc.v1.worker.ProcessReply")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub output: Output,
}
