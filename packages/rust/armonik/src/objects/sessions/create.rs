use crate::api::v3;

use super::super::TaskOptions;

/// Request for creating session.
#[derive(Debug, Clone, Default)]
pub struct SessionCreateRequest {
    /// Default tasks options for tasks in the session.
    pub default_task_option: TaskOptions,
    /// List of partitions allowed during the session.
    pub partition_ids: Vec<String>,
}

impl From<SessionCreateRequest> for v3::sessions::CreateSessionRequest {
    fn from(value: SessionCreateRequest) -> Self {
        Self {
            default_task_option: value.default_task_option.into(),
            partition_ids: value.partition_ids,
        }
    }
}

impl From<v3::sessions::CreateSessionRequest> for SessionCreateRequest {
    fn from(value: v3::sessions::CreateSessionRequest) -> Self {
        Self {
            default_task_option: value.default_task_option.into(),
            partition_ids: value.partition_ids,
        }
    }
}

super::super::impl_convert!(SessionCreateRequest : Option<v3::sessions::CreateSessionRequest>);

/// Reply after session creation.
/// We have this reply in case of success.
/// When the session creation is not successful, there is an rpc exception.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionCreateResponse {
    /// Session id of the created session if successful
    pub session_id: String,
}

impl From<SessionCreateResponse> for v3::sessions::CreateSessionReply {
    fn from(value: SessionCreateResponse) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

impl From<v3::sessions::CreateSessionReply> for SessionCreateResponse {
    fn from(value: v3::sessions::CreateSessionReply) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(SessionCreateResponse : Option<v3::sessions::CreateSessionReply>);
