use super::super::{DataChunk, TaskError};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub session_id: String,
    pub result_id: String,
}

super::super::impl_convert!(
    struct Request = v3::ResultRequest {
        session_id = session,
        result_id,
    }
);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Response {
    DataChunk(DataChunk),
    TaskError(TaskError),
    NotCompleted(String),
}

impl Default for Response {
    fn default() -> Self {
        Self::NotCompleted(Default::default())
    }
}

impl From<Response> for v3::submitter::ResultReply {
    fn from(value: Response) -> Self {
        match value {
            Response::DataChunk(chunk) => Self {
                r#type: Some(v3::submitter::result_reply::Type::Result(chunk.into())),
            },
            Response::TaskError(error) => Self {
                r#type: Some(v3::submitter::result_reply::Type::Error(error.into())),
            },
            Response::NotCompleted(msg) => Self {
                r#type: Some(v3::submitter::result_reply::Type::NotCompletedTask(msg)),
            },
        }
    }
}

impl From<v3::submitter::ResultReply> for Response {
    fn from(value: v3::submitter::ResultReply) -> Self {
        match value.r#type {
            Some(v3::submitter::result_reply::Type::Result(chunk)) => Self::DataChunk(chunk.into()),
            Some(v3::submitter::result_reply::Type::Error(error)) => Self::TaskError(error.into()),
            Some(v3::submitter::result_reply::Type::NotCompletedTask(msg)) => {
                Self::NotCompleted(msg)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Response : v3::submitter::ResultReply);
