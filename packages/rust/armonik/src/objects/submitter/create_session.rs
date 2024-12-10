use crate::api::v3;

use super::super::TaskOptions;

/// Request for creating session.
#[derive(Debug, Clone, Default)]
pub struct Request {
    /// Default tasks options for tasks in the session.
    pub default_task_options: TaskOptions,
    /// List of partitions allowed during the session.
    pub partition_ids: Vec<String>,
}

super::super::impl_convert!(
    struct Request = v3::submitter::CreateSessionRequest {
        default_task_options = option default_task_option,
        partition_ids,
    }
);

/// Reply after session creation.
/// We have this reply in case of success.
/// When the session creation is not successful, there is an rpc exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Session id of the created session if successful
    pub session_id: String,
}

super::super::impl_convert!(
    struct Response = v3::submitter::CreateSessionReply {
        session_id,
    }
);
