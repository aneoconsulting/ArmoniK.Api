use super::super::{SessionStatus, TaskOptions};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.sessions.SessionRaw")]
pub struct Raw {
    pub session_id: String,
    pub status: SessionStatus,
    pub client_submission: bool,
    pub worker_submission: bool,
    pub partition_ids: Vec<String>,
    #[armonik(rename = "options")]
    pub default_task_options: TaskOptions,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub cancelled_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub closed_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub purged_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub deleted_at: Option<prost_types::Timestamp>,
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub duration: Option<prost_types::Duration>,
}
