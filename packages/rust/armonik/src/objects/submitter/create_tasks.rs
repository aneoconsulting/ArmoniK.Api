use super::super::{DataChunk, InitTaskRequest, TaskOptions, TaskRequest};
use crate::utils::IntoCollection;

use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct SmallRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
    pub task_requests: Vec<TaskRequest>,
}

super::super::impl_convert!(
    struct SmallRequest = v3::submitter::CreateSmallTaskRequest {
        session_id,
        option task_options,
        list task_requests,
    }
);

#[derive(Debug, Clone, Default)]
pub struct InitRequest {
    pub session_id: String,
    pub task_options: Option<TaskOptions>,
}

super::super::impl_convert!(
    struct InitRequest = v3::submitter::create_large_task_request::InitRequest {
        session_id,
        option task_options,
    }
);

#[derive(Debug, Clone, Default)]
pub enum LargeRequest {
    #[default]
    Invalid,
    InitRequest(InitRequest),
    InitTaskRequest(InitTaskRequest),
    DataChunk(DataChunk),
}

impl From<LargeRequest> for v3::submitter::CreateLargeTaskRequest {
    fn from(value: LargeRequest) -> Self {
        match value {
            LargeRequest::Invalid => Self { r#type: None },
            LargeRequest::InitRequest(request) => Self {
                r#type: Some(v3::submitter::create_large_task_request::Type::InitRequest(
                    request.into(),
                )),
            },
            LargeRequest::InitTaskRequest(request) => Self {
                r#type: Some(v3::submitter::create_large_task_request::Type::InitTask(
                    request.into(),
                )),
            },
            LargeRequest::DataChunk(chunk) => Self {
                r#type: Some(v3::submitter::create_large_task_request::Type::TaskPayload(
                    chunk.into(),
                )),
            },
        }
    }
}

impl From<v3::submitter::CreateLargeTaskRequest> for LargeRequest {
    fn from(value: v3::submitter::CreateLargeTaskRequest) -> Self {
        match value.r#type {
            Some(v3::submitter::create_large_task_request::Type::InitRequest(request)) => {
                Self::InitRequest(request.into())
            }
            Some(v3::submitter::create_large_task_request::Type::InitTask(request)) => {
                Self::InitTaskRequest(request.into())
            }
            Some(v3::submitter::create_large_task_request::Type::TaskPayload(chunk)) => {
                Self::DataChunk(chunk.into())
            }
            None => Self::Invalid,
        }
    }
}

super::super::impl_convert!(req LargeRequest : v3::submitter::CreateLargeTaskRequest);

#[derive(Debug, Clone)]
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

impl From<Status> for v3::submitter::create_task_reply::CreationStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::TaskInfo {
                task_id,
                expected_output_keys,
                data_dependencies,
                payload_id,
            } => Self {
                status: Some(
                    v3::submitter::create_task_reply::creation_status::Status::TaskInfo(
                        v3::submitter::create_task_reply::TaskInfo {
                            task_id,
                            expected_output_keys,
                            data_dependencies,
                            payload_id,
                        },
                    ),
                ),
            },
            Status::Error(msg) => Self {
                status: Some(v3::submitter::create_task_reply::creation_status::Status::Error(msg)),
            },
        }
    }
}

impl From<v3::submitter::create_task_reply::CreationStatus> for Status {
    fn from(value: v3::submitter::create_task_reply::CreationStatus) -> Self {
        match value.status {
            Some(v3::submitter::create_task_reply::creation_status::Status::TaskInfo(status)) => {
                Self::TaskInfo {
                    task_id: status.task_id,
                    expected_output_keys: status.expected_output_keys,
                    data_dependencies: status.data_dependencies,
                    payload_id: status.payload_id,
                }
            }
            Some(v3::submitter::create_task_reply::creation_status::Status::Error(msg)) => {
                Self::Error(msg)
            }
            None => Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    Status(Vec<Status>),
    Error(String),
}

impl Default for Response {
    fn default() -> Self {
        Self::Error(Default::default())
    }
}

impl From<Response> for v3::submitter::CreateTaskReply {
    fn from(value: Response) -> Self {
        match value {
            Response::Status(status) => Self {
                response: Some(
                    v3::submitter::create_task_reply::Response::CreationStatusList(
                        v3::submitter::create_task_reply::CreationStatusList {
                            creation_statuses: status.into_collect(),
                        },
                    ),
                ),
            },
            Response::Error(msg) => Self {
                response: Some(v3::submitter::create_task_reply::Response::Error(msg)),
            },
        }
    }
}

impl From<v3::submitter::CreateTaskReply> for Response {
    fn from(value: v3::submitter::CreateTaskReply) -> Self {
        match value.response {
            Some(v3::submitter::create_task_reply::Response::CreationStatusList(status)) => {
                Self::Status(status.creation_statuses.into_collect())
            }
            Some(v3::submitter::create_task_reply::Response::Error(msg)) => Self::Error(msg),
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Response : v3::submitter::CreateTaskReply);
