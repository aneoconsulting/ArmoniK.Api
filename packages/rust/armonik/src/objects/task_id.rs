use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskId")]
pub struct TaskId {
    #[armonik(rename = "session")]
    pub session_id: String,
    #[armonik(rename = "task")]
    pub task_id: String,
}

super::impl_convert!(
    struct TaskId = v3::TaskId {
        session_id = session,
        task_id = task,
    }
);
