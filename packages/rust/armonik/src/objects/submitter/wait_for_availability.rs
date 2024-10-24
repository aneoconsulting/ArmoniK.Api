use super::super::TaskError;

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
pub enum Response {
    Ok,
    TaskError(TaskError),
    NotCompleted(String),
}

impl Default for Response {
    fn default() -> Self {
        Self::NotCompleted(Default::default())
    }
}

impl From<Response> for v3::submitter::AvailabilityReply {
    fn from(value: Response) -> Self {
        match value {
            Response::Ok => Self {
                r#type: Some(v3::submitter::availability_reply::Type::Ok(v3::Empty {})),
            },
            Response::TaskError(error) => Self {
                r#type: Some(v3::submitter::availability_reply::Type::Error(error.into())),
            },
            Response::NotCompleted(msg) => Self {
                r#type: Some(v3::submitter::availability_reply::Type::NotCompletedTask(
                    msg,
                )),
            },
        }
    }
}

impl From<v3::submitter::AvailabilityReply> for Response {
    fn from(value: v3::submitter::AvailabilityReply) -> Self {
        match value.r#type {
            Some(v3::submitter::availability_reply::Type::Ok(_)) => Self::Ok,
            Some(v3::submitter::availability_reply::Type::Error(error)) => {
                Self::TaskError(error.into())
            }
            Some(v3::submitter::availability_reply::Type::NotCompletedTask(msg)) => {
                Self::NotCompleted(msg)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Response : v3::submitter::AvailabilityReply);
