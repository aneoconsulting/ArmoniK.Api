use super::super::{DataChunk, InitTaskRequest, TaskOptions, TaskRequest};

#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSmallTaskRequest")]
pub struct SmallRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
    pub task_requests: Vec<TaskRequest>,
}

#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest.InitRequest")]
pub struct InitRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
}

#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.CreateLargeTaskRequest",
    oneof = "type"
)]
pub enum LargeRequest {
    #[default]
    Invalid,
    InitRequest(InitRequest),
    #[armonik(rename = "init_task")]
    InitTaskRequest(InitTaskRequest),
    #[armonik(rename = "task_payload")]
    DataChunk(DataChunk),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.CreateTaskReply.CreationStatus",
    oneof = "Status"
)]
pub enum Status {
    TaskInfo {
        /// Unique ID of the created task.
        task_id: String,
        /// Unique ID of the result that will be used as expected output. Results should already exist.
        expected_output_keys: Vec<String>,
        /// Unique ID of the result that will be used as data dependency. Results should already exist.
        data_dependencies: Vec<String>,
        /// Unique ID of the result that will be used as payload. Result associated to the payload is created implicitly.
        payload_id: String,
    },
    Error(String),
}

impl Default for Status {
    fn default() -> Self {
        Self::Error(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.CreateTaskReply",
    oneof = "Response"
)]
pub enum Response {
    /// The creation statuses, one per task creation request.
    #[armonik(
        rename = "creation_status_list",
        with = "crate::codec::adapters::VecWrapper<1>"
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
