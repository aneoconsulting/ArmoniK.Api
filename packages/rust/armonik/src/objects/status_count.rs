use super::TaskStatus;

#[armonik_macros::message("armonik.api.grpc.v1.StatusCount")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCount {
    pub status: TaskStatus,
    pub count: i32,
}
