use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub filter: super::TaskFilter,
    pub stop_on_first_task_error: bool,
    pub stop_on_first_task_cancellation: bool,
}

super::super::impl_convert!(
    struct Request = v3::submitter::WaitRequest {
        filter = option filter,
        stop_on_first_task_error,
        stop_on_first_task_cancellation,
    }
);

pub type Response = super::super::Count;
