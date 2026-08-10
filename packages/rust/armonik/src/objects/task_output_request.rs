#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.TaskOutputRequest")]
pub struct TaskOutputRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}
