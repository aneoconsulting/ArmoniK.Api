use super::TaskRequestHeader;

use crate::api::v3;

#[derive(Debug, Clone)]
pub enum InitTaskRequest {
    Header(TaskRequestHeader),
    LastTask,
}

impl Default for InitTaskRequest {
    fn default() -> Self {
        Self::Header(Default::default())
    }
}

impl From<InitTaskRequest> for v3::InitTaskRequest {
    fn from(value: InitTaskRequest) -> Self {
        match value {
            InitTaskRequest::Header(header) => Self {
                r#type: Some(v3::init_task_request::Type::Header(header.into())),
            },
            InitTaskRequest::LastTask => Self {
                r#type: Some(v3::init_task_request::Type::LastTask(true)),
            },
        }
    }
}

impl From<v3::InitTaskRequest> for InitTaskRequest {
    fn from(value: v3::InitTaskRequest) -> Self {
        match value.r#type {
            Some(v3::init_task_request::Type::Header(header)) => Self::Header(header.into()),
            Some(v3::init_task_request::Type::LastTask(_)) => Self::LastTask,
            None => Default::default(),
        }
    }
}
