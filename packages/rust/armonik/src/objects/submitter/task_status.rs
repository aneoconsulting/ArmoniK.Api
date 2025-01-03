use std::collections::HashMap;

use crate::api::v3;

use super::super::TaskStatus;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub task_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::submitter::GetTaskStatusRequest {
        list task_ids,
    }
);

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    pub statuses: HashMap<String, TaskStatus>,
}

impl From<Response> for v3::submitter::GetTaskStatusReply {
    fn from(value: Response) -> Self {
        Self {
            id_statuses: value
                .statuses
                .into_iter()
                .map(
                    |(id, status)| v3::submitter::get_task_status_reply::IdStatus {
                        task_id: id,
                        status: status as i32,
                    },
                )
                .collect(),
        }
    }
}

impl From<v3::submitter::GetTaskStatusReply> for Response {
    fn from(value: v3::submitter::GetTaskStatusReply) -> Self {
        Self {
            statuses: value
                .id_statuses
                .into_iter()
                .map(|id_status| (id_status.task_id, id_status.status.into()))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::submitter::GetTaskStatusReply);
