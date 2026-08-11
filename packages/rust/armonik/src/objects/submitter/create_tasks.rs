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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatus")]
pub enum Status {
    #[armonik(inline)]
    TaskInfo {
        task_id: String,
        expected_output_keys: Vec<String>,
        data_dependencies: Vec<String>,
        payload_id: String,
    },
    Error(String),
}

impl Default for Status {
    fn default() -> Self {
        Self::Error(Default::default())
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateTaskReply")]
pub enum Response {
    /// The creation statuses, one per task creation request.
    #[armonik(
        rename = "creation_status_list",
        with = "crate::codec::adapters::Wrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatusList"
    )]
    Status(Vec<Status>),
    /// The error message when all the task creations failed.
    Error(String),
}

impl Default for Response {
    fn default() -> Self {
        Self::Status(vec![])
    }
}
