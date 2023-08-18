use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskOutputRequest {
    pub session: String,
    pub task_id: String,
}

super::impl_convert!(
    struct TaskOutputRequest = v3::TaskOutputRequest {
        session,
        task_id,
    }
);
