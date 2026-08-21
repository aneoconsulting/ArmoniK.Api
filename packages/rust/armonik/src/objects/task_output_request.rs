#[armonik_macros::message("armonik.api.grpc.v1.TaskOutputRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskOutputRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub task_id: String,
}
