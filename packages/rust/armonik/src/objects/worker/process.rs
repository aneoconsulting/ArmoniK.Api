use super::super::{Configuration, Output, TaskOptions};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

super::super::impl_convert!(
    struct Request = v3::worker::ProcessRequest {
        communication_token,
        session_id,
        task_id,
        task_options = option task_options,
        list expected_output_keys,
        payload_id,
        list data_dependencies,
        data_folder,
        configuration = option configuration,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub output: Output,
}

super::super::impl_convert!(
    struct Response = v3::worker::ProcessReply {
        output = option output,
    }
);
