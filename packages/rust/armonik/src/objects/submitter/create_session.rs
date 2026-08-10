use super::super::TaskOptions;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSessionRequest")]
pub struct Request {
    pub partition_ids: Vec<String>,
    #[armonik(rename = "default_task_option")]
    pub default_task_options: TaskOptions,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSessionReply")]
pub struct Response {
    pub session_id: String,
}
