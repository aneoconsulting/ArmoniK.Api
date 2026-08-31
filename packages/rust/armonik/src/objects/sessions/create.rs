use super::super::TaskOptions;

#[armonik_macros::message("armonik.api.grpc.v1.sessions.CreateSessionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub partition_ids: Vec<String>,
    #[armonik(rename = "default_task_option")]
    pub default_task_options: TaskOptions,
}

#[armonik_macros::message("armonik.api.grpc.v1.sessions.CreateSessionReply")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub session_id: String,
}
