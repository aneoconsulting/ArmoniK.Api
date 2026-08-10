#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.TaskId")]
pub struct TaskId {
    #[armonik(rename = "session")]
    pub session_id: String,
    #[armonik(rename = "task")]
    pub task_id: String,
}
