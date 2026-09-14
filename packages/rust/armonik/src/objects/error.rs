use super::TaskStatus;

#[armonik_macros::message("armonik.api.grpc.v1.Error")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Error {
    pub task_status: TaskStatus,
    #[armonik(rename = "detail")]
    pub details: String,
}
