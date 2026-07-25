#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub session_id: String,
    pub task_id: String,
}

super::super::impl_convert!(
    struct Request = crate::TaskOutputRequest {
        session_id,
        task_id,
    }
);

pub type Response = super::super::Output;
