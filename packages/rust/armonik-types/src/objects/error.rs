use super::TaskStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Error")]
pub struct Error {
    pub task_status: TaskStatus,
    #[armonik(rename = "detail")]
    pub details: String,
}
