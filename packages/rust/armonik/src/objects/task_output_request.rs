#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskOutputRequest")]
pub struct TaskOutputRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}
