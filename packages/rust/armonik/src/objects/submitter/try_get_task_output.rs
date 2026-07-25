/// Request for retrieving a task output, standing for the
/// `TaskOutputRequest` message the stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub session_id: String,
    pub task_id: String,
}

impl From<Request> for crate::TaskOutputRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            task_id: value.task_id,
        }
    }
}

impl From<crate::TaskOutputRequest> for Request {
    fn from(value: crate::TaskOutputRequest) -> Self {
        Self {
            session_id: value.session_id,
            task_id: value.task_id,
        }
    }
}

pub type Response = super::super::Output;
