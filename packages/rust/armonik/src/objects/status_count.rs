use super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.StatusCount")]
pub struct StatusCount {
    pub status: TaskStatus,
    pub count: i32,
}
