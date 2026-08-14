use super::super::{DataChunk, InitTaskRequest, TaskOptions, TaskRequest};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSmallTaskRequest")]
pub struct SmallRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
    pub task_requests: Vec<TaskRequest>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest.InitRequest")]
pub struct InitRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest")]
pub enum LargeRequest {
    #[default]
    Invalid,
    InitRequest(InitRequest),
    #[armonik(rename = "init_task")]
    InitTaskRequest(InitTaskRequest),
    #[armonik(rename = "task_payload")]
    DataChunk(DataChunk),
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatus")]
pub enum Status {
    /// No member set, which an empty `Error` is not.
    #[default]
    Invalid,
    #[armonik(inline)]
    TaskInfo {
        task_id: String,
        expected_output_keys: Vec<String>,
        data_dependencies: Vec<String>,
        payload_id: String,
    },
    Error(String),
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateTaskReply")]
pub enum Response {
    /// No member set, which an empty `Status` list is not.
    #[default]
    Invalid,
    /// The creation statuses, one per task creation request.
    #[armonik(
        rename = "creation_status_list",
        with = "crate::codec::adapters::Wrapper",
        absorbs = "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatusList"
    )]
    Status(Vec<Status>),
    /// The error message when all the task creations failed.
    Error(String),
}
