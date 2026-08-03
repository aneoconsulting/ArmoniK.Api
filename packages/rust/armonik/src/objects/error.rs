use super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Error")]
pub struct Error {
    pub task_status: TaskStatus,
    #[armonik(rename = "detail")]
    pub details: String,
}
