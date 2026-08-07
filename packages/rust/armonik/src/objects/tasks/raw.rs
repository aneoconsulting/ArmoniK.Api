use super::super::{TaskOptions, TaskStatus};
use super::Output;

/// A detailed task object.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.TaskDetailed")]
pub struct Raw {
    #[armonik(rename = "id")]
    pub task_id: String,
    pub session_id: String,
    pub owner_pod_id: String,
    pub initial_task_id: String,
    pub parent_task_ids: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub expected_output_ids: Vec<String>,
    pub retry_of_ids: Vec<String>,
    pub status: TaskStatus,
    pub status_message: String,
    pub options: TaskOptions,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub submitted_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub received_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub acquired_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub fetched_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub started_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub processed_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub ended_at: Option<prost_types::Timestamp>,
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub creation_to_end_duration: Option<prost_types::Duration>,
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub processing_to_end_duration: Option<prost_types::Duration>,
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub received_to_end_duration: Option<prost_types::Duration>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub pod_ttl: Option<prost_types::Timestamp>,
    pub output: Output,
    pub pod_hostname: String,
    pub payload_id: String,
    pub created_by: String,
}
