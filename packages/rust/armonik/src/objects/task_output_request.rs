use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskOutputRequest")]
pub struct TaskOutputRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}

super::impl_convert!(
    struct TaskOutputRequest = v3::TaskOutputRequest {
        session_id = session,
        task_id,
    }
);
