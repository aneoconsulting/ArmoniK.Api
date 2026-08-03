use super::super::TaskOptions;

/// Request for creating session.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSessionRequest")]
pub struct Request {
    /// Default tasks options for tasks in the session.
    #[armonik(rename = "default_task_option")]
    pub default_task_options: TaskOptions,
    /// List of partitions allowed during the session.
    pub partition_ids: Vec<String>,
}

/// Reply after session creation.
/// We have this reply in case of success.
/// When the session creation is not successful, there is an rpc exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.CreateSessionReply")]
pub struct Response {
    /// Session id of the created session if successful
    pub session_id: String,
}
