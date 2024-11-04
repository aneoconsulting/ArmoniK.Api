use std::collections::HashMap;

use crate::api::v3;

use super::super::ResultStatus;

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub session_id: String,
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::submitter::GetResultStatusRequest {
        session_id,
        list result_ids,
    }
);

#[derive(Debug, Clone, Default)]
pub struct Response {
    pub statuses: HashMap<String, ResultStatus>,
}

impl From<Response> for v3::submitter::GetResultStatusReply {
    fn from(value: Response) -> Self {
        Self {
            id_statuses: value
                .statuses
                .into_iter()
                .map(
                    |(id, status)| v3::submitter::get_result_status_reply::IdStatus {
                        result_id: id,
                        status: status as i32,
                    },
                )
                .collect(),
        }
    }
}

impl From<v3::submitter::GetResultStatusReply> for Response {
    fn from(value: v3::submitter::GetResultStatusReply) -> Self {
        Self {
            statuses: value
                .id_statuses
                .into_iter()
                .map(|id_status| (id_status.result_id, id_status.status.into()))
                .collect(),
        }
    }
}

super::super::impl_convert!(req Response : v3::submitter::GetResultStatusReply);
