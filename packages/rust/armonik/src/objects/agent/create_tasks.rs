use super::super::{DataChunk, InitTaskRequest, TaskOptions};
use crate::utils::IntoCollection;

use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitRequest {
    pub task_options: Option<TaskOptions>,
}

super::super::impl_convert!(
    struct InitRequest = v3::agent::create_task_request::InitRequest {
        option task_options,
    }
);

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Request {
    #[default]
    Invalid,
    InitRequest {
        communication_token: String,
        request: InitRequest,
    },
    InitTaskRequest {
        communication_token: String,
        request: InitTaskRequest,
    },
    DataChunk {
        communication_token: String,
        chunk: DataChunk,
    },
}

impl From<Request> for v3::agent::CreateTaskRequest {
    fn from(value: Request) -> Self {
        match value {
            Request::Invalid => Self {
                communication_token: Default::default(),
                r#type: None,
            },
            Request::InitRequest {
                communication_token,
                request,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::InitRequest(
                    request.into(),
                )),
            },
            Request::InitTaskRequest {
                communication_token,
                request,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::InitTask(
                    request.into(),
                )),
            },
            Request::DataChunk {
                communication_token,
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::TaskPayload(
                    chunk.into(),
                )),
            },
        }
    }
}

impl From<v3::agent::CreateTaskRequest> for Request {
    fn from(value: v3::agent::CreateTaskRequest) -> Self {
        match value.r#type {
            Some(v3::agent::create_task_request::Type::InitRequest(request)) => Self::InitRequest {
                communication_token: value.communication_token,
                request: request.into(),
            },
            Some(v3::agent::create_task_request::Type::InitTask(request)) => {
                Self::InitTaskRequest {
                    communication_token: value.communication_token,
                    request: request.into(),
                }
            }
            Some(v3::agent::create_task_request::Type::TaskPayload(chunk)) => Self::DataChunk {
                communication_token: value.communication_token,
                chunk: chunk.into(),
            },
            None => Self::Invalid,
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::CreateTaskRequest);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl From<Status> for v3::agent::create_task_reply::CreationStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::TaskInfo {
                task_id,
                expected_output_keys,
                data_dependencies,
                payload_id,
            } => Self {
                status: Some(
                    v3::agent::create_task_reply::creation_status::Status::TaskInfo(
                        v3::agent::create_task_reply::TaskInfo {
                            task_id,
                            expected_output_keys,
                            data_dependencies,
                            payload_id,
                        },
                    ),
                ),
            },
            Status::Error(msg) => Self {
                status: Some(v3::agent::create_task_reply::creation_status::Status::Error(msg)),
            },
        }
    }
}

impl From<v3::agent::create_task_reply::CreationStatus> for Status {
    fn from(value: v3::agent::create_task_reply::CreationStatus) -> Self {
        match value.status {
            Some(v3::agent::create_task_reply::creation_status::Status::TaskInfo(status)) => {
                Self::TaskInfo {
                    task_id: status.task_id,
                    expected_output_keys: status.expected_output_keys,
                    data_dependencies: status.data_dependencies,
                    payload_id: status.payload_id,
                }
            }
            Some(v3::agent::create_task_reply::creation_status::Status::Error(msg)) => {
                Self::Error(msg)
            }
            None => Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Response {
    Status {
        communication_token: String,
        statuses: Vec<Status>,
    },
    Error {
        communication_token: String,
        error: String,
    },
}

impl Default for Response {
    fn default() -> Self {
        Self::Error {
            communication_token: Default::default(),
            error: Default::default(),
        }
    }
}

impl From<Response> for v3::agent::CreateTaskReply {
    fn from(value: Response) -> Self {
        match value {
            Response::Status {
                communication_token,
                statuses,
            } => Self {
                communication_token,
                response: Some(v3::agent::create_task_reply::Response::CreationStatusList(
                    v3::agent::create_task_reply::CreationStatusList {
                        creation_statuses: statuses.into_collect(),
                    },
                )),
            },
            Response::Error {
                communication_token,
                error,
            } => Self {
                communication_token,
                response: Some(v3::agent::create_task_reply::Response::Error(error)),
            },
        }
    }
}

impl From<v3::agent::CreateTaskReply> for Response {
    fn from(value: v3::agent::CreateTaskReply) -> Self {
        match value.response {
            Some(v3::agent::create_task_reply::Response::CreationStatusList(status)) => {
                Self::Status {
                    communication_token: value.communication_token,
                    statuses: status.creation_statuses.into_collect(),
                }
            }
            Some(v3::agent::create_task_reply::Response::Error(error)) => Self::Error {
                communication_token: value.communication_token,
                error,
            },
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Response : v3::agent::CreateTaskReply);
