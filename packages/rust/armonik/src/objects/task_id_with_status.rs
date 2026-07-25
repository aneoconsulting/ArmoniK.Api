use crate::api::v3;

use super::{TaskId, TaskStatus};

#[derive(Debug, Clone, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskIdWithStatus")]
pub struct TaskIdWithStatus {
    pub task_id: TaskId,
    pub status: TaskStatus,
}

super::impl_convert!(
    struct TaskIdWithStatus = v3::TaskIdWithStatus {
        task_id = option task_id,
        status = enum status,
    }
);
